use crate::system::cpu::affinity::group::GroupAffinity;

#[derive(Debug)]
pub struct Thread {
    pub thread_id: u32,
    pub affinity: GroupAffinity,
}

impl Thread {
    /// Constructor for Thread
    pub fn new(thread_id: u32, affinity: GroupAffinity) -> Self {
        Self {
            thread_id,
            affinity,
        }
    }
}
