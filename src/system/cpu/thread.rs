use std::{cell::RefCell, time::Instant};

use windows::Win32::{
    Foundation::FILETIME,
    System::Threading::{GetCurrentThread, GetThreadTimes},
};

use crate::{
    pawn_io::intel_msr::{with_affinity, GroupAffinity},
    system::sensor::sensor::{SensorImpl, SensorKind, SensorTarget},
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
    index: usize,
}

impl ThreadLoadSensor {
    pub fn new(affinity: GroupAffinity) -> Self {
        let bit_index = affinity.mask.trailing_zeros() as usize;
        let flat_index = affinity.group as usize * MAX_PROC_PER_GROUP + bit_index;

        println!(
            "Creating ThreadLoadSensor: group={}, mask=0x{:X}, flat_index={}",
            affinity.group, affinity.mask, flat_index
        );

        Self {
            affinity,
            prev_state: RefCell::new(None),
            index: flat_index,
        }
    }

    fn read_times(&self) -> Result<(u64, u64), String> {
        with_affinity(&self.affinity, || {
            let thread = unsafe { GetCurrentThread() };
            let mut creation = FILETIME::default();
            let mut exit = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();

            let ok =
                unsafe { GetThreadTimes(thread, &mut creation, &mut exit, &mut kernel, &mut user) }
                    .is_ok();
            if !ok {
                return Err("GetThreadTimes failed".into());
            }

            let kernel_ns =
                ((kernel.dwHighDateTime as u64) << 32 | kernel.dwLowDateTime as u64) * 100;
            let user_ns = ((user.dwHighDateTime as u64) << 32 | user.dwLowDateTime as u64) * 100;

            Ok((kernel_ns, user_ns))
        })
    }
}

impl SensorImpl for ThreadLoadSensor {
    fn read(&self, _parameters: &Option<Vec<f32>>) -> Result<f32, String> {
        let (kernel_ns, user_ns) = self.read_times()?;
        let now = Instant::now();
        let mut prev = self.prev_state.borrow_mut();

        let load_percent = if let Some((prev_kernel, prev_user, prev_time)) = *prev {
            let delta_ns = (kernel_ns + user_ns).saturating_sub(prev_kernel + prev_user);
            let elapsed_ns = now.duration_since(prev_time).as_nanos() as u64;
            if elapsed_ns == 0 {
                0.0
            } else {
                // CPU load fraction = time spent on CPU / elapsed wall-clock time
                let frac = (delta_ns as f64) / (elapsed_ns as f64);
                frac.min(1.0).max(0.0) * 100.0 // clamp 0..100%
            }
        } else {
            0.0 // first read, no previous data
        };

        // store new state
        *prev = Some((kernel_ns, user_ns, now));

        Ok(load_percent as f32)
    }

    fn kind(&self) -> SensorKind {
        SensorKind::Utilization
    }

    fn target(&self) -> SensorTarget {
        SensorTarget::Cpu
    }
}
