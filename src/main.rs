use std::{thread, time::Duration};

use crate::{
    pawn_io::intel_msr::{self, GroupAffinity, IntelMsr},
    system::{
        cpu::cpu::IntelCpuTempSensor,
        sensor::sensor::{SensorImpl, SensorKind},
        system::System,
    },
};

mod pawn_io;
mod system;

fn main() -> Result<(), String> {
    let mut s = System::new();
    // let intel_driver = s.insert_driver(|| IntelMsr::new());

    let _ = s.discover();

    for _ in 0..3 {
        for sensor in &s.sensors {
            if (sensor.kind() == SensorKind::Utilization) {
                println!(
                    "{}: {}, target: {:?}, kind: {:?}",
                    sensor.id,
                    sensor.read()?,
                    sensor.target(),
                    sensor.kind()
                );
            }
            // let _ = sensor.read()?;
        }

        thread::sleep(Duration::from_secs(1));
    }

    // let x = IntelCpuTempSensor {
    //     driver: IntelMsr::new(),
    //     affinity: GroupAffinity { mask: 1, group: 0 },
    // };

    // let t = x.read(&Some(vec![100f32, 1f32]));

    // println!("{:#?}", t);

    s.close();

    Ok(())
}
