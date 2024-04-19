use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::stream::StreamExt;
use r2r::autoware_adapi_v1_msgs::msg::ResponseStatus;
use r2r::autoware_adapi_v1_msgs::srv::ChangeOperationMode;
use r2r::autoware_auto_control_msgs::msg::{
    AckermannControlCommand, AckermannLateralCommand, LongitudinalCommand,
};
use r2r::builtin_interfaces::msg::Time;
use r2r::qos::QosProfile;
use r2r::{Clock, ClockType};
use tokio::runtime::Runtime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = r2r::Context::create()?;
    let mut node = r2r::Node::create(ctx, "server", "")?;
    let mut service =
        node.create_service::<ChangeOperationMode::Service>("/api/operation_mode/change_to_stop")?;

    let mut service_2 = node.create_service::<ChangeOperationMode::Service>(
        "/api/operation_mode/change_to_autonomous",
    )?;
    let rt = Runtime::new()?;

    let publisher = node
        .create_publisher::<r2r::autoware_auto_control_msgs::msg::AckermannControlCommand>(
            "/control/command/control_cmd",
            QosProfile::default(),
        )
        .unwrap();

    let gear_publisher = node
        .create_publisher::<r2r::autoware_auto_vehicle_msgs::msg::GearCommand>(
            "/control/command/gear_cmd",
            QosProfile::default(),
        )
        .unwrap();
    let mut clock = Clock::create(ClockType::RosTime)?;

    let is_stopped = Arc::new(AtomicBool::new(true));

    {
        let is_stopped = is_stopped.clone();

        rt.spawn(async move {
            loop {
                match service.next().await {
                    Some(req) => {
                        println!("got stop");
                        let good = String::new();
                        let k = ResponseStatus {
                            success: true,
                            code: 123,
                            message: good,
                        };

                        is_stopped.store(true, Ordering::SeqCst);

                        let msg = ChangeOperationMode::Response { status: k };
                        req.respond(msg).expect("could not send service response");
                    }
                    None => break,
                }
            }
        });
    }

    {
        let is_stopped = is_stopped.clone();
        rt.spawn(async move {
            loop {
                match service_2.next().await {
                    Some(req) => {
                        println!("got start");
                        let good = String::new();
                        let k = ResponseStatus {
                            success: true,
                            code: 123,
                            message: good,
                        };

                        is_stopped.store(false, Ordering::SeqCst);

                        let msg = ChangeOperationMode::Response { status: k };
                        req.respond(msg).expect("could not send service response");
                    }
                    None => break,
                }
            }
        });
    }

    rt.spawn(async move {
        loop {
            let time = clock.get_now().unwrap();

            let speed = if is_stopped.load(Ordering::SeqCst) {
                0.0
            } else {
                20.0
            };

            let ros_time = Time {
                sec: time.as_secs() as i32,
                nanosec: time.subsec_nanos(),
            };
            let cntrol = AckermannControlCommand {
                stamp: ros_time.clone(),
                lateral: AckermannLateralCommand {
                    stamp: ros_time.clone(),
                    steering_tire_angle: 0.0,
                    steering_tire_rotation_rate: 0.0,
                },
                longitudinal: LongitudinalCommand {
                    stamp: ros_time.clone(),
                    speed,
                    acceleration: 0.0,
                    jerk: 0.0,
                },
            };
            publisher.publish(&cntrol).unwrap();

            let gear = r2r::autoware_auto_vehicle_msgs::msg::GearCommand {
                stamp: ros_time.clone(),
                command: if is_stopped.load(Ordering::SeqCst) {
                    22
                } else {
                    2
                },
            };
            gear_publisher.publish(&gear).unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });

    loop {
        node.spin_once(std::time::Duration::from_millis(5));
    }
}
