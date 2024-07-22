use cute_lights::{discover_lights, CuteResult};
use std::thread::sleep;
use std::time::Duration;

#[tokio::main]
async fn main() -> CuteResult<()> {
    let mut lights = discover_lights().await;
    println!("Found {} lights", lights.len());
    let mut state = true;

    loop {

        for light in lights.iter_mut() {
            println!("Setting light {} to {}", light.name(), state);
            light.set_on(state).await?;
            light.set_color(255, 0, 0).await?;


            state = !state;
            sleep(Duration::from_secs(2));
        }
    }
}
