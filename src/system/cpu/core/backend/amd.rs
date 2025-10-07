use std::sync::Arc;

use crate::system::{cpu::core::backend::CoreBackend, kernal_driver::KernelDriver};

#[derive(Debug)]
pub struct AmdCoreBackend {
    driver: Arc<KernelDriver>,
}

impl CoreBackend for AmdCoreBackend {
    fn read_temp(&self) -> Result<f32, String> {
        Err("Not implmented".into())
    }
}

impl AmdCoreBackend {
    pub fn new(driver: Arc<KernelDriver>) -> Self {
        Self { driver }
    }
}
