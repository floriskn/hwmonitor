use crate::_src::system::cpu::affinity::group::GroupAffinity;

use super::thread::thread::Thread;

#[derive(Debug)]
pub struct Core {
    pub core_id: u32,
    pub threads: Vec<Thread>,
}

impl Core {
    /// Constructor for Core
    pub fn new(core_id: u32) -> Self {
        Self {
            core_id,
            threads: Vec::new(),
        }
    }
}
