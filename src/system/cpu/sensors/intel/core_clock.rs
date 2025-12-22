use std::{cell::RefCell, rc::Rc};

use x86::msr::IA32_PERF_STATUS;

use crate::{
    drivers::pawn_io::{
        intel_msr::{IntelMsr, IntelMsrError},
        pawn_io::PawnIoError,
    },
    system::{
        cpu::{
            affinity::group::GroupAffinity,
            backends::common::time_stamp_counter_backend::TimeStampCounterBackend,
            intel::micro_architecture::MicroArchitecture,
        },
        sensor::{SensorError, SensorImpl, SensorKind, SensorTarget},
    },
};

#[derive(Debug)]
pub struct IntelCoreClockSensor {
    pub driver: Rc<RefCell<IntelMsr>>,
    pub backend: Rc<RefCell<TimeStampCounterBackend>>,
    pub time_stamp_counter_multiplier: f64,
    pub affinity: GroupAffinity,
    pub micro_architecture: MicroArchitecture,
}

impl SensorImpl for IntelCoreClockSensor {
    fn read(&self, _: &Option<Vec<f32>>) -> Result<Option<f32>, SensorError> {
        // --- Hardware Note on Shared Voltage Rails ---
        // On most consumer Intel CPUs (like the i9-10850K or Alder/Raptor Lake),
        // all cores share a single VccIA voltage rail.
        //
        // While we read the VID (Voltage ID) per core with specific affinity,
        // the values will appear nearly identical because the Voltage Regulator
        // follows the "Highest Wins" policy (it supplies the maximum voltage
        // requested by any active core to the entire rail).
        //
        // Slight differences between readings are expected because sensors are
        // sampled sequentially; a core's frequency or load may shift in the
        // microseconds between individual MSR read calls.
        let tsc_freq = self.backend.borrow().time_stamp_counter_frequency;

        // 1. Try to read the MSR
        let msr_read = self
            .driver
            .borrow()
            .read_msr_affinity(IA32_PERF_STATUS, &self.affinity);

        // 2. Handle the result with specific fallback for IoControlFailed
        let (eax, _) = match msr_read {
            Ok(val) => val,
            Err(e) => match e {
                // SPECIFIC FALLBACK: If IoControl fails, return TSC frequency as the clock
                IntelMsrError::DriverError(PawnIoError::IoControlFailed { .. }) => {
                    return Ok(Some(tsc_freq as f32));
                }
                // CRITICAL ERRORS: These still stop the sensor
                IntelMsrError::AffinityError(ae) => {
                    return Err(SensorError::HardwareError(format!(
                        "Affinity switch failed: {ae}"
                    )));
                }
                IntelMsrError::DriverNotLoaded(_) => {
                    return Err(SensorError::HardwareError("MSR Driver not loaded".into()));
                }
                // OTHER ERRORS: Catch-all for other DriverError variants
                _ => {
                    return Err(SensorError::ReadFailure(format!("MSR Read failed: {e:?}")));
                }
            },
        };

        let bus_clock = tsc_freq / self.time_stamp_counter_multiplier;

        let value = match self.micro_architecture {
            MicroArchitecture::Nehalem => (eax & 0xff) as f64 * bus_clock,
            MicroArchitecture::Airmont
            | MicroArchitecture::AlderLake
            | MicroArchitecture::ArrowLake
            | MicroArchitecture::Broadwell
            | MicroArchitecture::CannonLake
            | MicroArchitecture::CometLake
            | MicroArchitecture::Goldmont
            | MicroArchitecture::GoldmontPlus
            | MicroArchitecture::Haswell
            | MicroArchitecture::IceLake
            | MicroArchitecture::IvyBridge
            | MicroArchitecture::JasperLake
            | MicroArchitecture::KabyLake
            | MicroArchitecture::LunarLake
            | MicroArchitecture::MeteorLake
            | MicroArchitecture::RaptorLake
            | MicroArchitecture::RocketLake
            | MicroArchitecture::SandyBridge
            | MicroArchitecture::Silvermont
            | MicroArchitecture::Skylake
            | MicroArchitecture::TigerLake
            | MicroArchitecture::SapphireRapids
            | MicroArchitecture::ElkhartLake
            | MicroArchitecture::Tremont => ((eax >> 8) & 0xff) as f64 * bus_clock,
            _ => {
                let ratio = ((eax >> 8) & 0x1f) as f64 + (0.5 * ((eax >> 14) & 1) as f64);
                ratio * bus_clock
            }
        };

        Ok(Some(value as f32))
    }

    fn kind(&self) -> SensorKind {
        SensorKind::Clock
    }

    fn target(&self) -> SensorTarget {
        SensorTarget::Cpu
    }
}
