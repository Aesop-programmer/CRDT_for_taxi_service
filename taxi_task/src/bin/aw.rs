use anyhow::{bail, ensure, Result};
use clap::Parser;
use futures::{self, FutureExt, Stream, StreamExt};
use inquire::Text;
use itertools::{chain, Itertools};
use r2r::{
    autoware_adapi_v1_msgs::{
        msg::RouteState,
        srv::{ChangeOperationMode, ClearRoute, SetRoutePoints},
    },
    geometry_msgs::msg::{Point, Pose, Quaternion},
    std_msgs::msg::Header,
    Client, Clock, ClockType, QosProfile,
};
use std::{fmt::Debug, path::PathBuf, pin::Pin, time::Duration};

pub type Subscriber<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

macro_rules! spawn {
    ($fut:expr) => {{
        tokio::spawn($fut).map(|result| -> Result<_> { result? })
    }};
}
macro_rules! spawn_blocking {
    ($func:expr) => {{
        tokio::task::spawn_blocking($func).map(|result| -> Result<_> { result? })
    }};
}

/// The interactive shell that assigns passenger tasks.
#[derive(Debug, Parser)]
struct Args {
    /// Zenoh configuration file.
    #[clap(long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Parser)]
enum Command {
    Stop,
    SetGoal,
    SetRoute,
    ClearRoute,
    Auto,
    Exit,
}

struct AutowareApiClient {
    set_route_points: Client<SetRoutePoints::Service>,
    clear_route: Client<ClearRoute::Service>,
    change_to_stop: Client<ChangeOperationMode::Service>,
    change_to_auto: Client<ChangeOperationMode::Service>,
}

struct AutowareApiSubscribers {
    route_state: Subscriber<RouteState>,
}

impl AutowareApiClient {
    async fn stop(&mut self) -> Result<ChangeOperationMode::Response> {
        let msg = ChangeOperationMode::Request {};
        let req = self.change_to_stop.request(&msg)?;
        let res = req.await?;
        Ok(res)
    }

    async fn clear_route(&mut self) -> Result<ClearRoute::Response> {
        let msg = ClearRoute::Request {};
        let req = self.clear_route.request(&msg)?;
        let res = req.await?;
        Ok(res)
    }

    async fn auto(&mut self) -> Result<ChangeOperationMode::Response> {
        let msg = ChangeOperationMode::Request {};
        let req = self.change_to_auto.request(&msg)?;
        let res = req.await?;
        Ok(res)
    }

    async fn set_route(
        &mut self,
        goal: Pose,
        waypoints: Vec<Pose>,
        clock: &mut Clock,
    ) -> Result<SetRoutePoints::Response> {
        let msg = SetRoutePoints::Request {
            header: Header {
                stamp: Clock::to_builtin_time(&clock.get_now()?),
                frame_id: "map".to_string(),
            },
            goal,
            waypoints,
            ..SetRoutePoints::Request::default()
        };

        let req = self.set_route_points.request(&msg)?;
        let res = req.await?;
        Ok(res)
    }

    async fn set_goal(
        &mut self,
        goal: Pose,
        waypoints: Vec<Pose>,
        clock: &mut Clock,
    ) -> Result<()> {
        let Self {
            set_route_points,
            clear_route,
            change_to_stop,
            change_to_auto,
        } = self;

        // stop car
        let msg = ChangeOperationMode::Request {};
        let req = change_to_stop.request(&msg)?;
        let res = req.await?;
        eprintln!("{res:#?}");
        if !res.status.success {
            return Ok(());
        }

        // clear route
        let msg = ClearRoute::Request {};
        let req = clear_route.request(&msg)?;
        let res = req.await?;
        eprintln!("{res:#?}");
        if !res.status.success {
            return Ok(());
        }

        // set route
        let msg = SetRoutePoints::Request {
            header: Header {
                stamp: Clock::to_builtin_time(&clock.get_now()?),
                frame_id: "map".to_string(),
            },
            goal,
            waypoints,
            ..SetRoutePoints::Request::default()
        };
        let req = set_route_points.request(&msg)?;
        let res = req.await?;
        eprintln!("{res:#?}");
        if !res.status.success {
            return Ok(());
        }

        // change to auto
        let msg = ChangeOperationMode::Request {};
        let req = change_to_auto.request(&msg)?;
        let res = req.await?;
        eprintln!("{res:#?}");
        if !res.status.success {
            return Ok(());
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Args::parse();

    // ROS
    let ctx = r2r::Context::create()?;
    let mut node = r2r::Node::create(ctx, "car", "")?;
    let clock = Clock::create(ClockType::RosTime)?;

    let route_state = node.subscribe::<RouteState>("/api/routing/state", QosProfile::default())?;

    let subs = AutowareApiSubscribers {
        route_state: Box::pin(route_state),
    };
    let client = AutowareApiClient {
        set_route_points: node
            .create_client::<SetRoutePoints::Service>("/api/routing/set_route_points")?,
        clear_route: node.create_client::<ClearRoute::Service>("/api/routing/clear_route")?,
        change_to_stop: node
            .create_client::<ChangeOperationMode::Service>("/api/operation_mode/change_to_stop")?,
        change_to_auto: node.create_client::<ChangeOperationMode::Service>(
            "/api/operation_mode/change_to_autonomous",
        )?,
    };

    let input_task = spawn!(shell(client, clock));
    let watch_task = spawn!(watch_subscribers(subs));
    let spin_task = spawn_blocking!(move || -> Result<()> {
        loop {
            node.spin_once(Duration::from_micros(5));
        }
    });

    futures::try_join!(input_task, spin_task, watch_task)?;

    Ok(())
}

async fn shell(mut client: AutowareApiClient, mut clock: Clock) -> Result<()> {
    'cmd_loop: loop {
        // The input loop repeats until a valid user request is
        // complete.
        let cmd: Command = loop {
            // Wait for the user to press enter.
            let Some(line) = Text::new(">").prompt_skippable()? else {
                break 'cmd_loop;
            };
            let tokens = chain!([""], line.split_whitespace());

            match Command::try_parse_from(tokens) {
                Ok(cmd) => break cmd,
                Err(err) => {
                    eprintln!("{err}");
                }
            }
        };

        match cmd {
            Command::SetGoal => {
                let (goal, waypoints) = ask_goal()?;
                client.set_goal(goal, waypoints, &mut clock).await?;
            }
            Command::SetRoute => {
                let (goal, waypoints) = ask_goal()?;
                let res = client.set_route(goal, waypoints, &mut clock).await?;
                println!("{res:#?}");
            }
            Command::ClearRoute => {
                let res = client.clear_route().await?;
                println!("{res:#?}");
            }
            Command::Auto => {
                let res = client.auto().await?;
                println!("{res:#?}");
            }
            Command::Stop => {
                let res = client.stop().await?;
                println!("{res:#?}");
            }
            Command::Exit => break,
        }

        // services.set_goal(goal, waypoints).await?;
    }

    Ok(())
}

async fn watch_subscribers(subs: AutowareApiSubscribers) -> Result<()> {
    let AutowareApiSubscribers { route_state } = subs;

    futures::try_join!(watch_subscriber(route_state),)?;

    Ok(())
}

async fn watch_subscriber<T>(sub: Subscriber<T>) -> Result<()>
where
    T: Debug,
{
    sub.for_each(|msg| async move {
        println!("{msg:#?}");
    })
    .await;
    Ok(())
}

fn ask_goal() -> Result<(Pose, Vec<Pose>)> {
    let goal = loop {
        let text = match Text::new("Goal:").prompt() {
            Ok(text) => text,
            Err(err) => {
                eprintln!("{err}");
                continue;
            }
        };

        if text.is_empty() {
            continue;
        }

        match parse_pose(&text) {
            Ok(pose) => break pose,
            Err(err) => eprintln!("{err}"),
        }
    };

    let waypoints: Vec<_> = (0..)
        .map(|id| loop {
            let text = match Text::new(&format!("Waypoint {id}: ")).prompt() {
                Ok(text) => text,
                Err(err) => {
                    eprintln!("{err}");
                    continue;
                }
            };

            if text.is_empty() {
                return None;
            }

            match parse_pose(&text) {
                Ok(pose) => break Some(pose),
                Err(err) => eprintln!("{err}"),
            }
        })
        .take_while(|pose| pose.is_some())
        .flatten()
        .collect();

    Ok((goal, waypoints))
}

fn parse_pose(input: &str) -> Result<Pose> {
    let values: Vec<f64> = input
        .split_ascii_whitespace()
        .map(|token| token.parse::<f64>())
        .try_collect()?;

    ensure!(values.len() == 7, "invalid pose");

    let &[x, y, z, i, j, k, w] = values.as_slice() else {
        bail!("invalid pose");
    };

    Ok(Pose {
        position: Point { x, y, z },
        orientation: Quaternion {
            x: i,
            y: j,
            z: k,
            w,
        },
    })
}
