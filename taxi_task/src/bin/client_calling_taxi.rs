use clap::Parser;
use inquire::Text;
use r2r::geometry_msgs::msg::{Point, Pose, Quaternion};
use std::path::PathBuf;
use taxi_task::CallTaxi;
use zenoh::prelude::r#async::*;
type Error = Box<dyn std::error::Error + Sync + Send>;

/// The interactive shell that assigns passenger tasks.
#[derive(Debug, Parser)]
struct Args {
    /// Zenoh configuration file.
    #[clap(long)]
    pub config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opts = Args::parse();

    // Zenoh init
    let session = {
        let config = match opts.config {
            Some(config_file) => Config::from_file(config_file)?,
            None => Config::default(),
        };
        zenoh::open(config).res().await?.into_arc()
    };
    let publisher = session.declare_publisher("rsu/calling_taxi").res().await?;

    // Start a task loop. One task ID per iteration.
    for task_id in 0.. {
        // The input loop repeats until a valid user request is
        // complete.
        let request = 'input_loop: loop {
            // Wait for the user to press enter.
            Text::new(&format!("Press [ENTER] to start Task {task_id}.\n")).prompt()?;

            // Ask for coordinates.
            let cur_location = read_position("Pick-up position (1..8):")?;
            let des_location = read_position("Goal position (1..8):")?;

            // Ask the user to confirm the input.
            loop {
                let answer = Text::new(&format!(
                    "Is this task correct?
task ID: {task_id}
Pick-up: location={}
Goal: location={}
Enter ([y]/n):",
                    cur_location, des_location
                ))
                .prompt()?;

                match answer.trim() {
                    "" | "y" => break,
                    "n" => continue 'input_loop,
                    _ => {
                        eprintln!("Please answer y or n.");
                    }
                }
            }
            let cur_location = match cur_location {
                1 => Pose {
                    position: Point {
                        x: 66.7734,
                        y: 3.68743,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: 0.996593,
                        w: 0.0824797,
                    },
                },
                2 => Pose {
                    position: Point {
                        x: -27.1465,
                        y: 2.29813,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: -0.999136,
                        w: 0.0415587,
                    },
                },
                3 => Pose {
                    position: Point {
                        x: -80.6396,
                        y: -0.649459,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: -0.999273,
                        w: 0.0381171,
                    },
                },
                4 => Pose {
                    position: Point {
                        x: -86.3321,
                        y: -4.08351,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: 0.00369801,
                        w: 0.999993,
                    },
                },
                5 => Pose {
                    position: Point {
                        x: -30.5858,
                        y: -1.69124,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: -0.0399045,
                        w: 0.999203,
                    },
                },
                6 => Pose {
                    position: Point {
                        x: 68.7716,
                        y: 1.69563,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: -0.0553005,
                        w: 0.99847,
                    },
                },
                7 => Pose {
                    position: Point {
                        x: 102.207,
                        y: 1.47179,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: -0.184385,
                        w: 0.982854,
                    },
                },
                8 => Pose {
                    position: Point {
                        x: 112.422,
                        y: 8.01776,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: -0.998625,
                        w: 0.052414,
                    },
                },
                _ => panic!("Invalid location"),
            };
            let des_location = match des_location {
                1 => Pose {
                    position: Point {
                        x: 66.7734,
                        y: 3.68743,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: 0.996593,
                        w: 0.0824797,
                    },
                },
                2 => Pose {
                    position: Point {
                        x: -27.1465,
                        y: 2.29813,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: -0.999136,
                        w: 0.0415587,
                    },
                },
                3 => Pose {
                    position: Point {
                        x: -80.6396,
                        y: -0.649459,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: -0.999273,
                        w: 0.0381171,
                    },
                },
                4 => Pose {
                    position: Point {
                        x: -86.3321,
                        y: -4.08351,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: 0.00369801,
                        w: 0.999993,
                    },
                },
                5 => Pose {
                    position: Point {
                        x: -30.5858,
                        y: -1.69124,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: -0.0399045,
                        w: 0.999203,
                    },
                },
                6 => Pose {
                    position: Point {
                        x: 68.7716,
                        y: 1.69563,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: -0.0553005,
                        w: 0.99847,
                    },
                },
                7 => Pose {
                    position: Point {
                        x: 102.207,
                        y: 1.47179,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: -0.184385,
                        w: 0.982854,
                    },
                },
                8 => Pose {
                    position: Point {
                        x: 112.422,
                        y: 8.01776,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: -0.998625,
                        w: 0.052414,
                    },
                },
                _ => panic!("Invalid location"),
            };
            break CallTaxi {
                task_id,
                cur_location,
                des_location,
            };
        };

        // Publish the request
        eprintln!("Publishing request...");
        let value = serde_json::to_value(request)?;
        publisher.put(value).res().await?;
        eprintln!("Done");
    }

    Ok(())
}

fn read_position(prompt: &str) -> Result<i32, Error> {
    // The loop runs until user gives a valid lat/lon position.
    loop {
        macro_rules! bail {
            () => {{
                eprintln!("incorrect position");
                continue;
            }};
        }

        // Get user input.
        let text = Text::new(prompt).prompt()?;

        // Check if it has two tokens, lat and lon string each.
        let tokens: Vec<_> = text.split_ascii_whitespace().collect();
        let [lat] = tokens.as_slice() else {
            bail!();
        };

        // Parse the tokens to floats.
        let Ok(location) = lat.parse() else { bail!() };

        return Ok(location);
    }
}
