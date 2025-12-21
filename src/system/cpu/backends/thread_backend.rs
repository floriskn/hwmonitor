use std::{cell::RefCell, mem::MaybeUninit};

use windows::{
    core::Error,
    Wdk::System::SystemInformation::{
        NtQuerySystemInformation, SystemProcessorPerformanceInformation,
    },
    Win32::{
        Foundation::{STATUS_INFO_LENGTH_MISMATCH, STATUS_SUCCESS},
        System::WindowsProgramming::SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION,
    },
};

use crate::system::backend::Backend;

#[derive(Debug, Clone)]
pub enum ThreadBackendError {
    Win32(windows::core::Error),
    IndexOutOfBounds { index: usize, max: usize },
}

impl std::fmt::Display for ThreadBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Win32(e) => write!(f, "Windows System Error: {e}"),
            Self::IndexOutOfBounds { index, max } => {
                write!(f, "Thread index {index} is out of bounds (count: {max})")
            }
        }
    }
}

impl std::error::Error for ThreadBackendError {}

#[derive(Debug)]
pub struct ThreadBackend {
    num_threads: usize,
    thread_loads: RefCell<Vec<Option<f32>>>,
    prev_idle: RefCell<Option<Vec<u64>>>,
    prev_total: RefCell<Option<Vec<u64>>>,
    last_error: RefCell<Option<Error>>,
}

impl ThreadBackend {
    pub fn new(num_threads: usize) -> Self {
        Self {
            num_threads,
            thread_loads: RefCell::new(vec![None; num_threads]),
            prev_idle: RefCell::new(None),
            prev_total: RefCell::new(None),
            last_error: RefCell::new(None),
        }
    }

    /// Returns the load for a thread, or the last Win32 error encountered
    pub fn get(&self, thread_index: usize) -> Result<Option<f32>, ThreadBackendError> {
        // If there is a pending error, return it
        if let Some(err) = self.last_error.borrow().clone() {
            return Err(ThreadBackendError::Win32(err));
        }

        self.thread_loads.borrow().get(thread_index).copied().ok_or(
            ThreadBackendError::IndexOutOfBounds {
                index: thread_index,
                max: self.num_threads,
            },
        )
    }

    unsafe fn get_times(&self) -> Result<(Vec<u64>, Vec<u64>), Error> {
        let mut num_elements = self.num_threads;
        let mut size =
            (size_of::<SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION>() * num_elements) as u32;

        // Initialize buffer with uninitialized memory
        let mut buffer =
            vec![MaybeUninit::<SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION>::uninit(); num_elements];
        let mut return_size = 0u32;

        let mut status = NtQuerySystemInformation(
            SystemProcessorPerformanceInformation,
            buffer.as_mut_ptr() as *mut _,
            size,
            &mut return_size,
        );

        // If initial size was wrong, resize and try exactly once more
        if status == STATUS_INFO_LENGTH_MISMATCH {
            num_elements =
                (return_size as usize) / size_of::<SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION>();
            buffer.resize(num_elements, MaybeUninit::uninit());
            size = return_size;

            status = NtQuerySystemInformation(
                SystemProcessorPerformanceInformation,
                buffer.as_mut_ptr() as *mut _,
                size,
                &mut return_size,
            );
        }

        status.ok()?;

        // Success: Now we assume the memory is initialized
        let perf_info = std::mem::transmute::<
            Vec<MaybeUninit<SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION>>,
            Vec<SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION>,
        >(buffer);

        let mut idle = Vec::with_capacity(perf_info.len());
        let mut total = Vec::with_capacity(perf_info.len());

        for info in perf_info {
            idle.push(info.IdleTime as u64);
            // KernelTime includes IdleTime on Windows, but the C# logic
            // provided sums Kernel + User for total.
            total.push((info.KernelTime + info.UserTime) as u64);
        }

        Ok((idle, total))
    }
}

impl Backend for ThreadBackend {
    fn update(&self) {
        // 1. Get new times or bail on error
        let (new_idle, new_total) = unsafe {
            match self.get_times() {
                Ok(times) => times,
                Err(e) => {
                    *self.last_error.borrow_mut() = Some(e);
                    *self.prev_idle.borrow_mut() = None;
                    *self.prev_total.borrow_mut() = None;
                    return;
                }
            }
        };

        // Clear error state on success
        *self.last_error.borrow_mut() = None;

        // 2. Get previous times or initialize and bail
        let (Some(prev_idle), Some(prev_total)) = (
            self.prev_idle.borrow().clone(),
            self.prev_total.borrow().clone(),
        ) else {
            *self.prev_idle.borrow_mut() = Some(new_idle);
            *self.prev_total.borrow_mut() = Some(new_total);
            return;
        };

        // 3. Verify threshold (guard clause)
        let min_diff = 100_000;
        let meets_threshold = new_total
            .iter()
            .zip(prev_total.iter())
            .all(|(n, p)| n.saturating_sub(*p) >= min_diff);

        if !meets_threshold {
            return;
        }

        // 4. Calculate and update loads
        let mut thread_loads = self.thread_loads.borrow_mut();

        for i in 0..self.num_threads.min(prev_idle.len()).min(new_idle.len()) {
            let delta_total = (new_total[i] - prev_total[i]) as f64;
            let delta_idle = (new_idle[i] - prev_idle[i]) as f64;

            if delta_total > 0.0 {
                let load = (1.0 - (delta_idle / delta_total).clamp(0.0, 1.0)) * 100.0;
                thread_loads[i] = Some(load as f32);
            }
        }

        // 5. Save state for next tick
        *self.prev_idle.borrow_mut() = Some(new_idle);
        *self.prev_total.borrow_mut() = Some(new_total);
    }
}
