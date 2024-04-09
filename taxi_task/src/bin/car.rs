use futures::{self, FutureExt};
use std::sync::Arc;
use taxi_task::{InfoTable, RequireTask};
use tokio::sync::Mutex;
use zenoh::prelude::r#async::*;
type Error = Box<dyn std::error::Error + Sync + Send>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // ROS
    // let ctx = r2r::Context::create()?;
    // let mut node = r2r::Node::create(ctx, "car", "")?;

    // Zenoh init
    let session = zenoh::open(Config::default()).res().await?.into_arc();

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
        // loop {
        //     node.spin_once(Duration::from_micros(5));
        // }

        Ok(())
    });

    // TODO: Implement commanding to Autoware.
    println!("Car is running");
    futures::try_join!(
        spawn!(merge_task(info_table.clone(), session.clone())),
        spin_task,
    )?;

    Ok(())
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
