use std::{cell::RefCell, rc::Rc, time::Instant};

use windows::Win32::{
    Foundation::FILETIME,
    System::Threading::{GetCurrentThread, GetThreadTimes},
};

use crate::{
    pawn_io::intel_msr::{with_affinity, GroupAffinity},
    system::{
        cpu::core::CpuLoadBackend,
        sensor::sensor::{SensorImpl, SensorKind, SensorTarget},
    },
};

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

#[cfg(target_pointer_width = "64")]
const MAX_PROC_PER_GROUP: usize = 64;

#[cfg(target_pointer_width = "32")]
const MAX_PROC_PER_GROUP: usize = 32;

#[derive(Debug)]
pub struct ThreadLoadSensor {
    pub affinity: GroupAffinity,
    prev_state: RefCell<Option<(u64, u64, Instant)>>, // kernel_ns, user_ns, timestamp
    backend: Rc<RefCell<CpuLoadBackend>>,
    index: usize,
}

impl ThreadLoadSensor {
    pub fn new(backend: &Rc<RefCell<CpuLoadBackend>>, affinity: GroupAffinity) -> Self {
        let bit_index = affinity.mask.trailing_zeros() as usize;
        let flat_index = affinity.group as usize * MAX_PROC_PER_GROUP + bit_index;

        // println!(
        //     "Creating ThreadLoadSensor: group={}, mask=0x{:X}, flat_index={}",
        //     affinity.group, affinity.mask, flat_index
        // );

        Self {
            backend: backend.clone(),
            affinity,
            prev_state: RefCell::new(None),
            index: flat_index,
        }
    }

    fn read_times(&self) -> Result<f32, String> {
        match self.backend.borrow().get(self.index) {
            Some(val) => Ok(val as f32), // convert f64 -> u64
            None => Err("No value found".to_string()),
        }
    }
}

impl SensorImpl for ThreadLoadSensor {
    fn read(&self, _parameters: &Option<Vec<f32>>) -> Result<f32, String> {
        self.read_times()
    }

    fn kind(&self) -> SensorKind {
        SensorKind::Utilization
    }

    fn target(&self) -> SensorTarget {
        SensorTarget::Cpu
    }
}
