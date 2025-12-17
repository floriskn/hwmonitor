use std::thread;

use windows::Win32::System::{
    SystemInformation::GROUP_AFFINITY,
    Threading::{GetCurrentThread, GetThreadGroupAffinity, SetThreadGroupAffinity},
};

use crate::pawn_io::intel_msr::GroupAffinity;

/// Run a closure for each group affinity in parallel,
/// while preserving the main thread's original affinity.
pub fn run_on_all_affinities<R, F>(affinities: Vec<GroupAffinity>, f: F) -> Result<Vec<R>, String>
where
    F: Fn(GroupAffinity) -> R + Send + Sync + 'static + Copy,
    R: Send + 'static,
{
    unsafe {
        // Save main thread's previous affinity
        let thread = GetCurrentThread();
        let mut prev: GROUP_AFFINITY = std::mem::zeroed();
        if !GetThreadGroupAffinity(thread, &mut prev).as_bool() {
            return Err("Failed to get previous thread affinity".into());
        }

        // Spawn threads for each affinity
        let handles: Vec<_> = affinities
            .into_iter()
            .map(|aff| {
                let f = f;
                thread::spawn(move || {
                    // Set this thread's affinity
                    let group = GROUP_AFFINITY {
                        Mask: aff.mask,
                        Group: aff.group,
                        Reserved: [0; 3],
                    };
                    let _ = SetThreadGroupAffinity(GetCurrentThread(), &group, None);

                    // Run the closure
                    f(aff)
                })
            })
            .collect();

        // Join threads and collect results
        let mut results = Vec::with_capacity(handles.len());
        for h in handles {
            results.push(h.join().map_err(|_| "Thread panicked".to_string())?);
        }

        // Restore main thread affinity
        if !SetThreadGroupAffinity(thread, &prev, None).as_bool() {
            return Err("Failed to restore main thread affinity".into());
        }

        Ok(results)
    }
}
