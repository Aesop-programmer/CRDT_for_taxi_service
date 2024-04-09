use clap::Parser;
use futures::{self, FutureExt};
use std::{path::PathBuf, sync::Arc, time::Duration};
use taxi_task::{CallTaxi, InfoTable, RequireTask, Task};
use tokio::sync::Mutex;
use zenoh::prelude::r#async::*;
type Error = Box<dyn std::error::Error + Sync + Send>;

/// The RSU node that demonstrates taxi task assignment algorithm.
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
    let mut node = r2r::Node::create(ctx, "rsu", "")?;

    // Zenoh init
    let session = {
        let config = match opts.config {
            Some(config_file) => Config::from_file(config_file)?,
            None => Config::default(),
        };
        zenoh::open(config).res().await?.into_arc()
    };

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
    println!("RSU is running");
    futures::try_join!(
        spawn!(deal_taxi_call(info_table.clone(), session.clone())),
        spawn!(merge_task(info_table.clone(), session.clone())),
        spin_task,
    )?;

    Ok(())
}
async fn deal_taxi_call(
    info_table: Arc<Mutex<InfoTable>>,
    session: Arc<Session>,
) -> Result<(), Error> {
    let subscriber = session.declare_subscriber("rsu/calling_taxi").res().await?;
    let publisher = session.declare_publisher("rsu/task_assign").res().await?;
    println!("RSU is waiting for taxi call");

    loop {
        let sample = subscriber.recv_async().await?;

        println!("Received taxi call");
        let value: serde_json::Value = sample.value.try_into()?;
        let call_taxi: CallTaxi = serde_json::from_value(value)?;
        println!("Received taxi call: {:?}", call_taxi);

        //update infotable
        let mut guard = info_table.lock().await;

        // TODO: check before insertion
        guard.task.insert(
            call_taxi.task_id,
            Task {
                task_id: call_taxi.task_id,
                cur_location: call_taxi.cur_location,
                des_location: call_taxi.des_location,
                timestamp: None,
                assigned_car: None,
            },
        );

        println!("InfoTable: {:?}", guard.task);

        //broadcast task
        let value = serde_json::to_value(call_taxi)?;
        publisher.put(value).res().await?;
    }
}

async fn merge_task(info_table: Arc<Mutex<InfoTable>>, session: Arc<Session>) -> Result<(), Error> {
    let subscriber = session.declare_subscriber("task_request").res().await?;

    loop {
        let require_task: RequireTask = {
            let sample = subscriber.recv_async().await?;
            let value: serde_json::Value = sample.value.try_into()?;
            serde_json::from_value(value)?
        };
        let mut guard = info_table.lock().await;
        guard.merge_task(require_task);
    }
}
