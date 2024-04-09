use inquire::Text;
use taxi_task::{CallTaxi, LatLon};
use zenoh::prelude::r#async::*;

type Error = Box<dyn std::error::Error + Sync + Send>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let session = zenoh::open(Config::default()).res().await?;
    let publisher = session.declare_publisher("rsu/calling_taxi").res().await?;

    // Start a task loop. One task ID per iteration.
    for task_id in 0.. {
        // The input loop repeats until a valid user request is
        // complete.
        let request = 'input_loop: loop {
            // Wait for the user to press enter.
            Text::new(&format!("Press [ENTER] to start Task {task_id}.\n")).prompt()?;

            // Ask for coordinates.
            let cur_location = read_position("Pick-up position (lat lon):")?;
            let des_location = read_position("Goal position (lat lon):")?;

            // Ask the user to confirm the input.
            loop {
                let answer = Text::new(&format!(
                    "Is this task correct?
task ID: {task_id}
Pick-up: lat={} lon={}
Goal: lat={} lon={}
Enter ([y]/n):",
                    cur_location.lat, cur_location.lon, des_location.lat, des_location.lon
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

fn read_position(prompt: &str) -> Result<LatLon, Error> {
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
        let [lat, lon] = tokens.as_slice() else {
            bail!();
        };

        // Parse the tokens to floats.
        let (Ok(lat), Ok(lon)) = (lat.parse(), lon.parse()) else {
            bail!()
        };

        return Ok(LatLon { lon, lat });
    }
}
