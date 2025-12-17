use std::{
    cell::RefCell,
    mem::{zeroed, MaybeUninit},
    slice,
};

use windows::{
    Wdk::System::SystemInformation::{
        NtQuerySystemInformation, SystemProcessorPerformanceInformation, SYSTEM_INFORMATION_CLASS,
    },
    Win32::Foundation::STATUS_SUCCESS,
};

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

#[derive(Debug)]
pub struct CpuLoadBackend {
    thread_loads: RefCell<Vec<f64>>,
    prev_idle: RefCell<Option<Vec<u64>>>,
    prev_total: RefCell<Option<Vec<u64>>>,
}

impl CpuLoadBackend {
    pub fn new(num_threads: usize) -> Self {
        Self {
            thread_loads: RefCell::new(vec![0.0; num_threads]),
            prev_idle: RefCell::new(None),
            prev_total: RefCell::new(None),
        }
    }

    /// Get per-thread load
    pub fn get(&self, thread_index: usize) -> Option<f64> {
        self.thread_loads.borrow().get(thread_index).copied()
    }

    unsafe fn get_windows_times(&self) -> Option<(Vec<u64>, Vec<u64>)> {
        let mut size = size_of::<SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION>() * 64;
        let mut buffer =
            vec![MaybeUninit::<SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION>::uninit(); 64];
        let mut return_size = 0u32;

        loop {
            // TODO: cleanup
            let status = NtQuerySystemInformation(
                SystemProcessorPerformanceInformation, // SystemProcessorPerformanceInformation
                buffer.as_mut_ptr() as *mut _,
                size as u32,
                &mut return_size,
            );

            if status.0 == 0xC0000004u32 as i32 {
                // STATUS_INFO_LENGTH_MISMATCH
                size = return_size as usize;
                buffer.resize(
                    size / size_of::<SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION>(),
                    MaybeUninit::uninit(),
                );
            } else if status.0 != 0 {
                // STATUS_SUCCESS
                return None;
            } else {
                break;
            }
        }

        let perf_infos: &[SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION] = slice::from_raw_parts(
            buffer.as_ptr() as *const _,
            return_size as usize / size_of::<SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION>(),
        );

        let mut idle = Vec::with_capacity(perf_infos.len());
        let mut total = Vec::with_capacity(perf_infos.len());

        for info in perf_infos {
            idle.push(info.idle_time as u64);
            total.push((info.kernel_time + info.user_time) as u64);
        }

        Some((idle, total))
    }
}

impl Drop for CpuLoadBackend {
    fn drop(&mut self) {
        println!("DROPPED ThreadBackend")
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION {
    idle_time: i64,
    kernel_time: i64,
    user_time: i64,
    dpc_time: i64,
    interrupt_time: i64,
    interrupt_count: u32,
}

impl Backend for CpuLoadBackend {
    fn update(&self) {
        let prev_idle = self.prev_idle.borrow().clone();
        let prev_total = self.prev_total.borrow().clone();

        let (new_idle, new_total) = unsafe {
            match self.get_windows_times() {
                Some(val) => val,
                None => return,
            }
        };

        if prev_idle.is_none() || prev_total.is_none() {
            *self.prev_idle.borrow_mut() = Some(new_idle);
            *self.prev_total.borrow_mut() = Some(new_total);
            return;
        }

        let prev_idle = prev_idle.unwrap();
        let prev_total = prev_total.unwrap();

        // Skip update if changes are too small
        let min_diff = 100_000;
        for i in 0..prev_total.len().min(new_total.len()) {
            if new_total[i] - prev_total[i] < min_diff {
                return;
            }
        }

        let mut total_idle = 0.0;
        let mut count = 0;
        let mut thread_loads = self.thread_loads.borrow_mut();

        for i in 0..thread_loads.len().min(prev_idle.len()).min(new_idle.len()) {
            let delta_total = (new_total[i] - prev_total[i]) as f64;
            let delta_idle = (new_idle[i] - prev_idle[i]) as f64;

            if delta_total == 0.0 {
                continue;
            }

            let load_percent = (1.0 - (delta_idle / delta_total).clamp(0.0, 1.0)) * 100.0;

            thread_loads[i] = (load_percent * 100.0).round() / 100.0; // keep 2 decimals
            count += 1;
        }

        let total_load = if count > 0 {
            let t = 1.0 - total_idle / count as f64;
            (t.clamp(0.0, 1.0) * 100.0 * 100.0).round() / 100.0
        } else {
            0.0
        };

        *self.prev_idle.borrow_mut() = Some(new_idle);
        *self.prev_total.borrow_mut() = Some(new_total);
    }
}
