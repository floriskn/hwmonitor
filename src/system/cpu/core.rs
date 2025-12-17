use crate::system::{cpu::thread::Thread, system::Backend};

#[derive(Debug)]
pub struct Core {
    pub core_id: u32,
    pub threads: Vec<Thread>,
}

impl Core {
    pub fn new(core_id: u32) -> Self {
        Self {
            core_id,
            threads: Vec::new(), // start empty
        }
    }
}

struct CpuLoadBackend {}

impl CpuLoadBackend {}

impl Drop for CpuLoadBackend {
    fn drop(&mut self) {
        println!("DROPPED ThreadBackend")
    }
}

impl Backend for CpuLoadBackend {
    fn update(&self) {
        todo!()
    }
}
