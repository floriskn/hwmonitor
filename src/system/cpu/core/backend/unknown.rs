use std::sync::Arc;

use crate::system::{cpu::core::backend::CoreBackend, kernal_driver::KernelDriver};

#[derive(Debug)]
pub struct UnknownCoreBackend {
    driver: Arc<KernelDriver>,
}

impl CoreBackend for UnknownCoreBackend {
    fn read_temp(&self) -> Result<f32, String> {
        Err("Not implmented".into())
    }
}

impl UnknownCoreBackend {
    pub fn new(driver: Arc<KernelDriver>) -> Self {
        Self { driver }
    }
}
