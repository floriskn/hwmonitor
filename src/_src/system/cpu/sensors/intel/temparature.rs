use std::{cell::RefCell, rc::Rc};

use x86::msr::IA32_PACKAGE_THERM_STATUS;

use crate::_src::{
    drivers::pawn_io::intel_msr::{IntelMsr, IntelMsrError},
    system::{
        cpu::affinity::group::GroupAffinity,
        sensor::{SensorError, SensorImpl, SensorKind, SensorTarget},
    },
};

#[derive(Debug)]
pub struct IntelCpuTempSensor {
    pub driver: Rc<RefCell<IntelMsr>>,
    pub affinity: GroupAffinity,
}

impl SensorImpl for IntelCpuTempSensor {
    fn read(&self, parameters: &Option<Vec<f32>>) -> Result<Option<f32>, SensorError> {
        // 1. Read the MSR and map IntelMsrError -> SensorError
        let (eax, _) = self
            .driver
            .borrow()
            .read_msr_affinity(IA32_PACKAGE_THERM_STATUS, &self.affinity)
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

        // 2. Check the Digital Readout Valid bit (Bit 31)
        // If this bit is 0, the temperature data in the MSR is not yet valid/stable.
        if (eax & 0x8000_0000) == 0 {
            return Err(SensorError::ReadFailure(
                "MSR Digital Readout bit is invalid".into(),
            ));
        }

        // 3. Process parameters
        let params = parameters.as_deref();
        let tj_max = params.and_then(|v| v.get(0)).copied().unwrap_or(100.0);
        let slope = params.and_then(|v| v.get(1)).copied().unwrap_or(1.0);

        // 4. Calculate Temperature
        // The value is a "Delta to TjMax", meaning the MSR stores how many degrees
        // below the maximum thermal limit the CPU currently is.
        let delta_t = ((eax & 0x007F_0000) >> 16) as f32;
        let temp = tj_max - (slope * delta_t);

        Ok(Some(temp))
    }

    fn kind(&self) -> SensorKind {
        SensorKind::Temperature
    }
    fn target(&self) -> SensorTarget {
        SensorTarget::Cpu
    }
}
