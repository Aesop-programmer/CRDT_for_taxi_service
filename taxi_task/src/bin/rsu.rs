use std::{time::{Duration,Instant}, str::SplitWhitespace, sync::Arc};
use futures;
use taxi_task::{InfoTable,CallTaxi,RequireTask};
use zenoh::prelude::r#async::{*, self};
use tokio::sync::Mutex;
type Error = Box<dyn std::error::Error +Sync + Send>;

#[tokio::main]
async fn main() -> Result<(),Error> {

    // ROS
    let ctx = r2r::Context::create()?;
    let mut node = r2r::Node::create(ctx, "rsu", "")?;
    // Zenoh init
    let session = zenoh::open(Config::default()).res().await?.into_arc();


    let mut spin_task = tokio::task::spawn_blocking(move || loop {
        node.spin_once(Duration::from_micros(5));
    });

    let info_table = InfoTable::new();
    let info_table = Arc::new(Mutex::new(info_table));

    


    Ok(())
}
async fn deal_taxi_call(info_table: Arc<Mutex<InfoTable>> , session: Arc<Session>) -> Result<(),Error> {
    let queryable = session.declare_queryable("/rsu/task_assign").complete(true).res().await?;
    loop{

    }
}

async fn merge_task(info_table: Arc<Mutex<InfoTable>>, session: Arc<Session>) -> Result<(),Error> {
    let subscriber = session.declare_subscriber("/tasktable").res().await?;
    loop{

    }
}