use std::sync::Arc;

use crate::system::{
    cpu::{backend::CpuBackend, group_affinity::GroupAffinity},
    kernal_driver::KernelDriver,
};

#[derive(Debug)]
pub struct AmdBackend {
    driver: Arc<KernelDriver>,
}

impl CpuBackend for AmdBackend {
    fn read_package_temp(&self, affinity: &GroupAffinity) -> Result<f32, String> {
        Err("Not implmented".into())
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

impl AmdBackend {
    pub fn new(driver: Arc<KernelDriver>) -> Self {
        Self { driver }
    }
}
