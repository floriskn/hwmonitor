use std::sync::Arc;

use crate::system::cpu::{backend::CpuBackend, thread::Thread};

pub struct Core {
    backend: Arc<dyn CpuBackend + Send + Sync>,
    pub core_id: u32,
    pub threads: Vec<Thread>,
}

impl Core {
    pub fn new(core_id: u32, backend: Arc<dyn CpuBackend + Send + Sync>) -> Self {
        Self {
            backend,
            core_id,
            threads: Vec::new(), // start empty
        }
    }

    pub fn temperature(&self) -> Option<f32> {
        self.backend.read_core_temp(self.core_id)
    }
}

impl std::fmt::Debug for Core {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Core")
            .field("core_id", &self.core_id)
            .field("threads", &self.threads)
            .finish()
    }
}
