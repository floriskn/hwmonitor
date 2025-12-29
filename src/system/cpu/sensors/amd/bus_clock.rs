use std::{cell::RefCell, rc::Rc};

use crate::{
    drivers::pawn_io::{
        amd_family_0f::{AmdFamily0F, AmdFamily0FError},
        pawn_io::PawnIoError,
    },
    system::{
        cpu::backends::common::time_stamp_counter_backend::TimeStampCounterBackend,
        sensor::{SensorError, SensorImpl, SensorKind, SensorTarget},
    },
};

#[derive(Debug)]
pub struct AmdBusClockSensor {
    pub backend: Rc<RefCell<TimeStampCounterBackend>>,
    pub driver: Rc<RefCell<AmdFamily0F>>,
}

impl SensorImpl for AmdBusClockSensor {
    fn read(&self, _: &Option<Vec<f32>>) -> Result<Option<f32>, SensorError> {
        let tsc_freq = self.backend.borrow().time_stamp_counter_frequency;

        const FIDVID_STATUS: u32 = 0xC0010042;

        // 1. Try to read the MSR
        let msr_read = self.driver.borrow().read_msr(FIDVID_STATUS);

        // 2. Handle the result with specific fallback for IoControlFailed
        let (eax, _) = match msr_read {
            Ok(val) => val,
            Err(e) => match e {
                // SPECIFIC FALLBACK: If IoControl fails, return TSC frequency as the clock
                AmdFamily0FError::DriverError(PawnIoError::IoControlFailed { .. }) => {
                    return Ok(Some(tsc_freq as f32));
                }
                // CRITICAL ERRORS: These still stop the sensor
                AmdFamily0FError::AffinityError(ae) => {
                    return Err(SensorError::HardwareError(format!(
                        "Affinity switch failed: {ae}"
                    )));
                }
                AmdFamily0FError::DriverNotLoaded(_) => {
                    return Err(SensorError::HardwareError("MSR Driver not loaded".into()));
                }
                // OTHER ERRORS: Catch-all for other DriverError variants
                _ => {
                    return Err(SensorError::ReadFailure(format!("MSR Read failed: {e:?}")));
                }
            },
        };

        // CurrFID can be found in eax bits 0-5, MaxFID in 16-21
        // 8-13 hold StartFID, we don't use that here.
        // let cur_mp = 0.5 * (((eax & 0x3F) + 8) as f64);
        let max_mp = 0.5 * (((eax >> 16 & 0x3F) + 8) as f64);

        Ok(Some((tsc_freq / max_mp) as f32))
    }

    fn kind(&self) -> SensorKind {
        SensorKind::Clock
    }

    fn target(&self) -> SensorTarget {
        SensorTarget::Cpu
    }
}
