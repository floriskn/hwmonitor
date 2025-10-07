use std::sync::Arc;

use crate::system::cpu::{core::backend::CoreBackend, thread::Thread};

pub struct Core {
    backend: Arc<dyn CoreBackend + Send + Sync>,
    pub core_id: u32,
    pub threads: Vec<Thread>,
}

impl Core {
    pub fn new(core_id: u32, backend: Arc<dyn CoreBackend + Send + Sync>) -> Self {
        Self {
            backend,
            core_id,
            threads: Vec::new(), // start empty
        }
    }

    pub fn temperature(&self) -> Option<f32> {
        match self.backend.read_temp() {
            Ok(temp) => Some(temp),
            Err(_) => None,
        }
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
