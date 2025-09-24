use std::sync::Arc;

use x86::msr::IA32_PACKAGE_THERM_STATUS;

use crate::system::{
    affinity::GroupAffinity, cpu::backend::CpuBackend, kernal_driver::KernelDriver,
};

#[derive(Debug)]
pub struct IntelBackend {
    driver: Arc<KernelDriver>,
}

impl CpuBackend for IntelBackend {
    fn read_package_temp(&self, package_id: u32) -> Result<f32, String> {
        let (eax, _) = self.driver.rdmsr_tx(
            IA32_PACKAGE_THERM_STATUS,
            GroupAffinity { group: 0, mask: 0 },
        )?;

        if (eax & 0x80000000) != 0 {
            let delta_t = ((eax & 0x007F0000) >> 16) as f32;
            let tj_max = 100f32;
            let t_slope = 1f32;

            return Ok(tj_max - t_slope * delta_t);
        }

        Err("Unknown value".into())
    }

    fn read_core_temp(&self, core_id: u32) -> Option<f32> {
        None
    }

    fn read_thread_load(&self, thread_id: u32) -> Option<f32> {
        None
    }

    fn read_power(&self, package_id: u32) -> Option<f32> {
        None
    }

    fn read_voltage(&self, core_id: u32) -> Option<f32> {
        None
    }
}

impl IntelBackend {
    pub fn new(driver: Arc<KernelDriver>) -> Self {
        Self { driver }
    }
}
