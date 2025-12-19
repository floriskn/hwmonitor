use std::thread;

use windows::Win32::System::{
    SystemInformation::GROUP_AFFINITY,
    Threading::{GetCurrentThread, GetThreadGroupAffinity, SetThreadGroupAffinity},
};

use super::{error::AffinityError, group::GroupAffinity};

pub fn run_on_all_affinities<R, F>(
    affinities: Vec<GroupAffinity>,
    f: F,
) -> Result<Vec<R>, AffinityError>
where
    F: Fn(GroupAffinity) -> R + Send + Sync + 'static + Copy,
    R: Send + 'static,
{
    unsafe {
        // Save main thread's previous affinity
        let thread = GetCurrentThread();
        let mut prev: GROUP_AFFINITY = std::mem::zeroed();
        if !GetThreadGroupAffinity(thread, &mut prev).as_bool() {
            return Err(AffinityError::GetThreadAffinityFailed);
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

                    let thread = GetCurrentThread();
                    if !SetThreadGroupAffinity(thread, &group, None).as_bool() {
                        return Err(AffinityError::SetThreadAffinityFailed);
                    }

                    // Run the closure
                    Ok(f(aff))
                })
            })
            .collect();

        // Join threads and collect results
        let mut results = Vec::with_capacity(handles.len());
        for h in handles {
            match h.join() {
                Ok(Ok(v)) => results.push(v),
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(AffinityError::ThreadPanic),
            }
        }

        // Restore main thread affinity
        if !SetThreadGroupAffinity(thread, &prev, None).as_bool() {
            return Err(AffinityError::SetThreadAffinityFailed);
        }

        Ok(results)
    }
}

/// Set thread affinity temporarily, run the closure, restore old affinity
pub fn with_affinity<F, R, E>(aff: &GroupAffinity, f: F) -> Result<R, E>
where
    F: FnOnce() -> Result<R, E>,
    E: From<AffinityError>, // The magic sauce
{
    unsafe {
        let thread = GetCurrentThread();

        let mut prev: GROUP_AFFINITY = std::mem::zeroed();
        if !GetThreadGroupAffinity(thread, &mut prev).as_bool() {
            return Err(E::from(AffinityError::GetThreadAffinityFailed));
        }

        let new_aff = GROUP_AFFINITY {
            Mask: aff.mask,
            Group: aff.group,
            Reserved: [0; 3],
        };

        if !SetThreadGroupAffinity(thread, &new_aff, Some(&mut prev)).as_bool() {
            return Err(E::from(AffinityError::SetThreadAffinityFailed));
        }

        // Run the inner function (e.g., read_msr)
        let result = f();

        // Restore affinity
        if !SetThreadGroupAffinity(thread, &prev, None).as_bool() {
            return Err(E::from(AffinityError::SetThreadAffinityFailed));
        }

        result
    }
}
