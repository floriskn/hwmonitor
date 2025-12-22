use std::{cell::RefCell, rc::Rc};

use x86::msr::IA32_PERF_STATUS;

use crate::{
    drivers::pawn_io::intel_msr::{IntelMsr, IntelMsrError},
    system::{
        cpu::affinity::group::GroupAffinity,
        sensor::{SensorError, SensorImpl, SensorKind, SensorTarget},
    },
};

#[derive(Debug)]
pub struct IntelCpuVoltageSensor {
    pub driver: Rc<RefCell<IntelMsr>>,
    pub affinity: Option<GroupAffinity>,
}

impl SensorImpl for IntelCpuVoltageSensor {
    fn read(&self, _parameters: &Option<Vec<f32>>) -> Result<Option<f32>, SensorError> {
        let msr_read_result = match &self.affinity {
            Some(affinity) => self
                .driver
                .borrow()
                .read_msr_affinity(IA32_PERF_STATUS, affinity),
            None => self.driver.borrow().read_msr(IA32_PERF_STATUS),
        };

        let (_, edx) = msr_read_result.map_err(|e| match e {
            IntelMsrError::AffinityError(ae) => {
                SensorError::HardwareError(format!("Affinity switch failed: {ae}"))
            }
            IntelMsrError::DriverNotLoaded(_) => {
                SensorError::HardwareError("MSR driver not loaded".into())
            }
            IntelMsrError::DriverError(pe) => {
                SensorError::ReadFailure(format!("MSR read failed: {pe:?}"))
            }
        })?;

        let digital_readout = (edx & 0xFFFF) as f32; // bits 15:0 of EDX

        if digital_readout == 0.0 {
            return Err(SensorError::ReadFailure(
                "MSR voltage readout is zero (invalid or unavailable)".into(),
            ));
        }

        let voltage = digital_readout / (1u32 << 13) as f32;

        Ok(Some(voltage))
    }

    fn kind(&self) -> SensorKind {
        SensorKind::Voltage
    }

    fn target(&self) -> SensorTarget {
        SensorTarget::Cpu
    }
}
