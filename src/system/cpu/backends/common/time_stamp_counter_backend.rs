use std::{
    arch::x86_64::_rdtsc,
    time::{Duration, Instant},
};

use crate::system::{
    backend::Backend,
    cpu::affinity::{error::AffinityError, group::GroupAffinity, utils::with_affinity},
};

#[derive(Debug)]
pub struct TimeStampCounterBackend {
    pub time_stamp_counter_frequency: f64,
    cpu_root_affinity: GroupAffinity,
    last_time: Option<Instant>,
    last_time_stamp_count: u64,
}

impl TimeStampCounterBackend {
    pub fn new(cpu_root_affinity: GroupAffinity) -> Self {
        let tsc_result: Result<(f64, f64), AffinityError> =
            with_affinity(&cpu_root_affinity, || Ok(Self::estimate_tsc_frequency()));

        let (estimated_time_stamp_counter_frequency, _) = match tsc_result {
            Ok(v) => v,
            Err(_) => (0.0, 0.0),
        };

        Self {
            cpu_root_affinity,
            last_time: None,
            last_time_stamp_count: 0,
            time_stamp_counter_frequency: estimated_time_stamp_counter_frequency,
        }
    }

    /// Estimates the TSC frequency by calling the inner function multiple times
    fn estimate_tsc_frequency() -> (f64, f64) {
        let mut frequency = 0.0;
        let mut error = f64::MAX;

        // preload the function (warm-up)
        Self::estimate_tsc_frequency_inner(0.0);
        Self::estimate_tsc_frequency_inner(0.0);

        for _ in 0..5 {
            let (f, e) = Self::estimate_tsc_frequency_inner(0.025);
            if e < error {
                error = e;
                frequency = f;
            }
            if error < 1e-4 {
                break;
            }
        }

        (frequency, error)
    }

    /// Estimates TSC frequency over a given time window (in seconds)
    fn estimate_tsc_frequency_inner(time_window: f64) -> (f64, f64) {
        // Convert to approximate duration
        let duration_ticks = Duration::from_secs_f64(time_window);

        // Small delay to match the C# ceil(0.001 * ticks)
        let delay = Duration::from_micros((0.001 * 1_000_000.0 * time_window) as u64);

        let now = Instant::now();
        let time_begin = now + delay;
        let time_end = time_begin + duration_ticks;

        // Wait until begin
        while Instant::now() < time_begin {}

        // read TSC and timestamp
        let count_begin = unsafe { _rdtsc() };
        let after_begin = Instant::now();

        // Wait until end
        while Instant::now() < time_end {}

        // read TSC and timestamp again
        let count_end = unsafe { _rdtsc() };
        let after_end = Instant::now();

        // Compute delta (seconds)
        let delta = time_end.duration_since(time_begin).as_secs_f64();

        // Frequency in MHz (1e-6 factor)
        let frequency = 1e-6 * ((count_end - count_begin) as f64 / delta);

        // Estimate relative error
        let begin_error = (after_begin.duration_since(time_begin).as_secs_f64()) / delta;
        let end_error = (after_end.duration_since(time_end).as_secs_f64()) / delta;
        let error = begin_error + end_error;

        (frequency, error)
    }
}

impl Backend for TimeStampCounterBackend {
    fn update(&mut self) {
        let measurement: Result<(Instant, u64, Instant), AffinityError> =
            with_affinity(&self.cpu_root_affinity, || {
                let first_time = Instant::now();

                let tsc = unsafe { _rdtsc() };

                let time = Instant::now();

                Ok((first_time, tsc, time))
            });

        let (first_time, time_stamp_count, time) = match measurement {
            Ok(v) => v,
            Err(_) => return,
        };

        let error = time.duration_since(first_time).as_secs_f64();

        // Only use data if measured accurately enough (max 0.1 ms)
        if error < 0.0001 {
            if let Some(last_time) = self.last_time {
                // Delta is the time between the current 'time' and the 'last_time'
                // This is exactly what (time - _lastTime) does in C#
                let delta = time.duration_since(last_time).as_secs_f64();

                // Ignore if the window is outside 0.5s to 2.0s
                if delta > 0.5 && delta < 2.0 {
                    let delta_tsc = time_stamp_count.wrapping_sub(self.last_time_stamp_count);

                    // MHz = cycles / (seconds * 10^6)
                    self.time_stamp_counter_frequency = (delta_tsc as f64) / (1e6 * delta);
                }
            }

            self.last_time_stamp_count = time_stamp_count;
            self.last_time = Some(time);
        }
    }
}
