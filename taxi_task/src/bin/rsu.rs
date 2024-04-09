use std::{time::{Duration,Instant}, str::SplitWhitespace, sync::Arc};
use futures;
use taxi_task::{InfoTable,CallTaxi,RequireTask,Task};
use zenoh::{prelude::r#async::{*, self}, info};
use tokio::sync::Mutex;
use futures::{stream::StreamExt, FutureExt, Stream};
type Error = Box<dyn std::error::Error +Sync + Send>;

#[tokio::main]
async fn main() -> Result<(),Error> {

    // ROS
    let ctx = r2r::Context::create()?;
    let mut node = r2r::Node::create(ctx, "rsu", "")?;
    // Zenoh init
    let session = zenoh::open(Config::default()).res().await?.into_arc();

    let info_table = InfoTable::new();
    let info_table = Arc::new(Mutex::new(info_table));
    
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
async fn deal_taxi_call(info_table: Arc<Mutex<InfoTable>> , session: Arc<Session>) -> Result<(),Error> {
    let subscriber = session.declare_subscriber("rsu/calling_taxi").res().await?;
    let publisher = session.declare_publisher("rsu/task_assign").res().await?;
    println!("RSU is waiting for taxi call");
    while let Ok(sample) = subscriber.recv_async().await{
        println!("Received taxi call");
        let value = sample.value.try_into()?;
        let call_taxi: CallTaxi = serde_json::from_value(value)?;
        println!("Received taxi call: {:?}", call_taxi);
        //update infotable
        let mut guard = info_table.lock().await;
        guard.task.push(Task{
            task_id: call_taxi.task_id,
            cur_location: call_taxi.cur_location,
            des_location: call_taxi.des_location,
            timestamp: std::u64::MAX,
            assigned_car: 0,
        });
        println!("InfoTable: {:?}", guard.task);
        //broadcast task
        let value = serde_json::to_value(call_taxi)?;
        publisher.put(value).res().await?;
        
    }
    Ok(())
}

async fn merge_task(info_table: Arc<Mutex<InfoTable>>, session: Arc<Session>) -> Result<(),Error> {
    let subscriber = session.declare_subscriber("task_request").res().await?;
    while let Ok(sample) = subscriber.recv_async().await{
        let value = sample.value.try_into()?;
        let require_task: RequireTask = serde_json::from_value(value)?;
        let mut guard = info_table.lock().await;
        guard.merge_task(require_task);
    }
    Ok(())
}