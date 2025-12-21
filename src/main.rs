#![allow(dead_code)]
#![allow(unused_imports)]

use std::{thread, time::Duration};

use crate::system::{cpu::cpu::Cpu, sensor::SensorKind, system::System};

mod drivers;
pub mod system;

fn main() -> Result<(), String> {
    let mut s = System::new();

    let _ = s.discover();

    for _ in 0..5 {
        s.update_backends();
        for sensor in &s.sensors {
            if sensor.kind() != SensorKind::Clock {
                continue;
            }

            let display_value = match sensor.read() {
                Ok(Some(val)) => val.to_string(),
                Ok(None) => "null".to_string(),
                Err(_) => "error".to_string(),
            };

            println!(
                "{}: {}, target: {:?}, kind: {:?}",
                sensor.id,
                display_value,
                sensor.target(),
                sensor.kind()
            );

            // }
            // let _ = sensor.read()?;
        }

        thread::sleep(Duration::from_secs(1));
    }

    s.close();

    Ok(())
}
