use std::sync::Arc;

use crate::system::cpu::{backend::CpuBackend, group_affinity::GroupAffinity};

pub struct Thread {
    pub thread_id: u32,
    pub affinity: GroupAffinity,
    backend: Arc<dyn CpuBackend + Send + Sync>,
}

impl Thread {
    /// Constructor for Thread
    pub fn new(
        thread_id: u32,
        affinity: GroupAffinity,
        backend: Arc<dyn CpuBackend + Send + Sync>,
    ) -> Self {
        Self {
            thread_id,
            affinity,
            backend,
        }
    }
}

impl std::fmt::Debug for Thread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Thread")
            .field("thread_id", &self.thread_id)
            .field("affinity", &self.affinity)
            .finish()
    }
}
