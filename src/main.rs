use std::{thread::sleep, time::Duration};

use crate::system::system::System;

mod system;

fn main() -> Result<(), String> {
    let system = System::builder().cpu().build()?;

    let binding = system.cpu.as_ref().unwrap();
    let cpu = binding.get(0).unwrap();

    for core in cpu.cores() {
        println!("Core {} temp: {:?}", core.core_id, core.temperature());
    }

    for _ in 0..10 {
        match cpu.package_temp() {
            Ok(value) => println!("Package temp: {}", value),
            Err(error) => println!("Error reading package temp: {}", error),
        }

        sleep(Duration::from_secs(1));
    }

    system.close()?;

    Ok(())
}
