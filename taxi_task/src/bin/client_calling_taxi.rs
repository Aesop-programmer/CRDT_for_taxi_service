use std::{time::{Duration,Instant}, str::SplitWhitespace, sync::Arc};
use futures;
use serde::de::value;
use taxi_task::{InfoTable,CallTaxi,RequireTask,Task};
use zenoh::{prelude::r#async::{*, self}, info};
use tokio::sync::Mutex;
use futures::{stream::StreamExt, FutureExt, Stream};
type Error = Box<dyn std::error::Error +Sync + Send>;
#[tokio::main]
async fn main() -> Result<(),Error> { 
    let session = zenoh::open(Config::default()).res().await?.into_arc();
    let publisher = session.declare_publisher("rsu/calling_taxi").res().await?;
    
    let call_taxi = CallTaxi{
        task_id: 1,
        cur_location: 1,
        des_location: 2,
    };
    let value = serde_json::to_value(call_taxi)?;
    publisher.put(value).res().await?;
    Ok(())
}