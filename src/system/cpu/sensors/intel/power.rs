use std::{cell::RefCell, rc::Rc, time::Instant};

use crate::{
    drivers::pawn_io::intel_msr::{IntelMsr, IntelMsrError},
    system::sensor::{SensorError, SensorImpl, SensorKind, SensorTarget},
};

#[derive(Debug)]
pub struct IntelCpuPowerSensor {
    pub index: u32,
    pub driver: Rc<RefCell<IntelMsr>>,
    pub last_energy_consumed: RefCell<u32>,
    pub last_energy_time: RefCell<Instant>,
    pub energy_units_multiplier: f32,
    pub value: RefCell<Option<f32>>,
}

impl SensorImpl for IntelCpuPowerSensor {
    fn read(&self, _parameters: &Option<Vec<f32>>) -> Result<Option<f32>, SensorError> {
        // 1. Read the MSR and map IntelMsrError -> SensorError
        let (eax, _) = self
            .driver
            .borrow()
            .read_msr(self.index)
            .map_err(|e| match e {
                // If the driver is missing or affinity fails, it's a Hardware/Read failure
                IntelMsrError::AffinityError(ae) => {
                    SensorError::HardwareError(format!("Affinity switch failed: {ae}"))
                }
                IntelMsrError::DriverNotLoaded(_) => {
                    SensorError::HardwareError("MSR Driver not loaded".into())
                }
                IntelMsrError::DriverError(pe) => {
                    SensorError::ReadFailure(format!("MSR Read failed: {pe:?}"))
                }
            })?;

        let energy_consumed = eax;
        let now = Instant::now();

        // Borrow last_energy_time and last_energy_consumed for interior mutability
        let mut last_time = self.last_energy_time.borrow_mut();
        let mut last_energy = self.last_energy_consumed.borrow_mut();
        let delta_time = now.duration_since(*last_time).as_secs_f32();
        if delta_time < 0.01 {
            return Ok(*self.value.borrow());
        }

        let delta_energy = energy_consumed.wrapping_sub(*last_energy);
        let power = self.energy_units_multiplier * delta_energy as f32 / delta_time;

        // Update interior fields
        *last_energy = energy_consumed;
        *last_time = now;
        *self.value.borrow_mut() = Some(power);

        Ok(Some(power))
    }

    fn kind(&self) -> SensorKind {
        SensorKind::Power
    }
    fn target(&self) -> SensorTarget {
        SensorTarget::Cpu
    }
}
