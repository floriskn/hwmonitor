use std::sync::Arc;

use x86::msr::IA32_PACKAGE_THERM_STATUS;

use crate::system::{
    cpu::{core::backend::CoreBackend, group_affinity::GroupAffinity},
    kernal_driver::KernelDriver,
};

const IA32_TEMPERATURE_TARGET: u32 = 0x01A2;

#[derive(Debug)]
pub struct IntelCoreBackend {
    driver: Arc<KernelDriver>,
    tj_max: f32,
    affinity: GroupAffinity,
}

impl CoreBackend for IntelCoreBackend {
    fn read_temp(&self) -> Result<f32, String> {
        let (eax, _) = self
            .driver
            .rdmsr_tx(IA32_PACKAGE_THERM_STATUS, &self.affinity)?;

        if (eax & 0x80000000) != 0 {
            let delta_t = ((eax & 0x007F0000) >> 16) as f32;
            let tj_max = self.tj_max;
            let t_slope = 1f32;

            return Ok(tj_max - t_slope * delta_t);
        }

        Err("Unknown value".into())
    }
}

impl IntelCoreBackend {
    pub fn new(driver: Arc<KernelDriver>, affinity: GroupAffinity) -> Self {
        // TODO: Not on every achritecture
        let tj_max = Self::get_tj_max_from_msr(&driver, &affinity).unwrap_or(100f32);

        println!("tj_max: {tj_max}");

        Self {
            driver,
            tj_max,
            affinity,
        }
    }

    pub fn get_tj_max_from_msr(
        driver: &Arc<KernelDriver>,
        affinity: &GroupAffinity,
    ) -> Result<f32, String> {
        let (eax, _) = driver.rdmsr_tx(IA32_TEMPERATURE_TARGET, affinity)?;

        return Ok(((eax >> 16) & 0xFF) as f32);
    }
}
