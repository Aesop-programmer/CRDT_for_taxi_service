use clap::Parser;
use futures::{self, FutureExt};
use inquire::Text;
use r2r::autoware_adapi_v1_msgs::srv::{ChangeOperationMode, ClearRoute, SetRoutePoints};
use r2r::geometry_msgs::msg::Pose;
use r2r::{Client, WrappedTypesupport};
use rand::prelude::*;
use std::{path::PathBuf, sync::Arc, time::Duration};
use taxi_task::{InfoTable, RequireTask, Task, VehicleState};
use tokio::sync::Mutex;
use unix_ts::{ts, Timestamp};
use zenoh::{prelude::r#async::*, subscriber};
type Error = Box<dyn std::error::Error + Sync + Send>;

/// The vehicle node that demonstrates taxi task assignment algorithm.
#[derive(Debug, Parser)]
struct Args {
    /// Zenoh configuration file.
    #[clap(long)]
    pub config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opts = Args::parse();

    // ROS
    let ctx = r2r::Context::create()?;
    let mut node = r2r::Node::create(ctx, "car", "")?;
    let set_route_points = Arc::new(Mutex::new(
        node.create_client::<SetRoutePoints::Service>("/api/routing/set_route_points")?,
    ));
    let clear_route = Arc::new(Mutex::new(
        node.create_client::<ClearRoute::Service>("/api/routing/clear_route")?,
    ));
    let change_to_stop = Arc::new(Mutex::new(
        node.create_client::<ChangeOperationMode::Service>("/api/operation_mode/change_to_stop")?,
    ));
    let change_to_auto = Arc::new(Mutex::new(
        node.create_client::<ChangeOperationMode::Service>(
            "/api/operation_mode/change_to_autonomous",
        )?,
    ));

    // Zenoh init
    let session = {
        let config = match opts.config {
            Some(config_file) => Config::from_file(config_file)?,
            None => Config::default(),
        };
        zenoh::open(config).res().await?.into_arc()
    };

    // car id from command line
    let request = 'input_loop: loop {
        Text::new(&format!("Press [ENTER] to login your car id.\n")).prompt()?;
        let car_id = read_car_id("your car id:")?;
        break car_id;
    };

    let vehicle_state = Arc::new(Mutex::new(VehicleState::new(request)));
    let info_table = Arc::new(Mutex::new(InfoTable::new()));

    macro_rules! spawn {
        ($fut:expr) => {{
            tokio::spawn($fut).map(|result| -> Result<_, Error> { result? })
        }};
    }
    macro_rules! spawn_blocking {
        ($func:expr) => {{
            tokio::task::spawn_blocking($func).map(|result| -> Result<_, Error> { result? })
        }};
    }

    let spin_task = spawn_blocking!(move || -> Result<(), Error> {
        loop {
            node.spin_once(Duration::from_micros(5));
        }
    });

    // TODO: Implement commanding to Autoware.
    println!("Car is running");
    futures::try_join!(
        spawn!(merge_task(
            info_table.clone(),
            session.clone(),
            vehicle_state.clone(),
            set_route_points.clone(),
            clear_route.clone(),
            change_to_stop.clone(),
            change_to_auto.clone()
        )),
        spawn!(listen_task(
            info_table.clone(),
            session.clone(),
            vehicle_state.clone(),
            set_route_points.clone(),
            clear_route.clone(),
            change_to_stop.clone(),
            change_to_auto.clone()
        )),
        spin_task,
    )?;

    Ok(())
}

fn read_car_id(prompt: &str) -> Result<u32, Error> {
    // The loop runs until user gives a valid lat/lon position.
    loop {
        macro_rules! bail {
            () => {{
                eprintln!("incorrect car_id");
                continue;
            }};
        }

        // Get user input.
        let text = Text::new(prompt).prompt()?;

        // Check if it has one tokens,
        let tokens: Vec<_> = text.split_ascii_whitespace().collect();
        let [car] = tokens.as_slice() else {
            bail!();
        };

        // Parse the tokens to floats.
        let Ok(car_id) = car.parse() else { bail!() };

        return Ok(car_id);
    }
}

async fn merge_task(
    info_table: Arc<Mutex<InfoTable>>,
    session: Arc<Session>,
    vehicle_state: Arc<Mutex<VehicleState>>,
    set_route: Arc<Mutex<Client<SetRoutePoints::Service>>>,
    clear_route: Arc<Mutex<Client<ClearRoute::Service>>>,
    change_to_stop: Arc<Mutex<Client<ChangeOperationMode::Service>>>,
    change_to_auto: Arc<Mutex<Client<ChangeOperationMode::Service>>>,
) -> Result<(), Error> {
    let subscriber = session.declare_subscriber("task_request").res().await?;
    let publisher = session.declare_publisher("task_request").res().await?;
    loop {
        let require_task: RequireTask = {
            let sample = subscriber.recv_async().await?;
            let value: serde_json::Value = sample.value.try_into()?;
            serde_json::from_value(value)?
        };
        //check if this require task complete with our car
        let mut guard_info_table = info_table.lock().await;
        guard_info_table.merge_task(require_task.clone());

        let mut guard_vehicle = vehicle_state.lock().await;
        // if the car fails competing for the task, it needs to be reset
        if guard_vehicle.assigned_task == Some(require_task.task_id) {
            let task = guard_info_table.task.get(&require_task.task_id).unwrap();
            if task.assigned_car != Some(guard_vehicle.car_id) {
                guard_vehicle.busy = false;
                guard_vehicle.assigned_task = None;
                for (key, value) in guard_info_table.task.iter_mut() {
                    if value.assigned_car == None {
                        guard_vehicle.busy = true;
                        guard_vehicle.assigned_task = Some(value.task_id);
                        let require_task = RequireTask {
                            task_id: value.task_id,
                            car_id: guard_vehicle.car_id,
                            timestamp: Timestamp::now().seconds() + (rand::random::<i64>() % 10),
                        };
                        let value = serde_json::to_value(require_task)?;
                        publisher.put(value).res().await?;
                        break;
                    }
                }
                // stop car
                let stop = change_to_stop.lock().await;
                let msg = ChangeOperationMode::Request {};
                let req = stop.request(&msg).unwrap();
                let res = req.await.unwrap();
                // clear route
                let clear = clear_route.lock().await;
                let msg = ClearRoute::Request {};
                let req = clear.request(&msg).unwrap();
                let res = req.await.unwrap();
                // if car does find new task
                if guard_vehicle.busy == true {
                    // set route
                    let set = set_route.lock().await;
                    let task = guard_info_table
                        .task
                        .get(&guard_vehicle.assigned_task.unwrap())
                        .unwrap();
                    let mut msg = SetRoutePoints::Request::default();
                    msg.goal = Pose::default(); //todo task.des_location fill location
                    msg.waypoints.push(Pose::default()); //todo task.cur_location
                    let req = set.request(&msg).unwrap();
                    let res = req.await.unwrap();

                    // start auto mode
                    let auto = change_to_auto.lock().await;
                    let msg = ChangeOperationMode::Request {};
                    let req = auto.request(&msg).unwrap();
                    let res = req.await.unwrap();
                }
            }
            //todo: change the car state in atuoware
        }
        eprint!("Receive task request: {:?}\n", require_task.clone());
        eprint!("Current task table: {:?}\n", guard_info_table.task.clone());
        eprint!("Current vehicle state: {:?}\n", guard_vehicle.clone());
    }
}

async fn listen_task(
    info_table: Arc<Mutex<InfoTable>>,
    session: Arc<Session>,
    vehicle_state: Arc<Mutex<VehicleState>>,
    set_route: Arc<Mutex<Client<SetRoutePoints::Service>>>,
    clear_route: Arc<Mutex<Client<ClearRoute::Service>>>,
    change_to_stop: Arc<Mutex<Client<ChangeOperationMode::Service>>>,
    change_to_auto: Arc<Mutex<Client<ChangeOperationMode::Service>>>,
) -> Result<(), Error> {
    let subscriber = session.declare_subscriber("rsu/task_assign").res().await?;
    let publisher = session.declare_publisher("task_request").res().await?;
    loop {
        let task: Task = {
            let sample = subscriber.recv_async().await?;
            let value: serde_json::Value = sample.value.try_into()?;
            serde_json::from_value(value)?
        };
        let mut guard_info_table = info_table.lock().await;
        if guard_info_table.task.contains_key(&task.task_id) {
            continue;
        } else {
            guard_info_table.task.insert(task.task_id, task.clone());
        }
        // check whether car is busy or not
        let mut guard_vehicle = vehicle_state.lock().await;
        if guard_vehicle.busy == false {
            guard_vehicle.busy = true;
            guard_vehicle.assigned_task = Some(task.task_id);

            let require_task = RequireTask {
                task_id: task.task_id,
                car_id: guard_vehicle.car_id,
                timestamp: Timestamp::now().seconds() + (rand::random::<i64>() % 10),
            };
            let value = serde_json::to_value(require_task)?;
            publisher.put(value).res().await?;

            //todo change the car state in autoware

            // set route
            let set = set_route.lock().await;
            let mut msg = SetRoutePoints::Request::default();
            msg.goal = Pose::default(); //todo task.des_location fill location
            msg.waypoints.push(Pose::default()); //todo task.cur_location
            let req = set.request(&msg).unwrap();
            let res = req.await.unwrap();
        }
        eprint!("Receive task assign: {:?}\n", task.clone());
        eprint!("Current task table: {:?}\n", guard_info_table.task.clone());
        eprint!("Current vehicle state: {:?}\n", guard_vehicle.clone());
    }

    Ok(())
}
