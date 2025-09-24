use std::thread;

use windows::Win32::{
    Foundation::ERROR_INSUFFICIENT_BUFFER,
    System::{
        SystemInformation::{
            GetLogicalProcessorInformationEx, RelationProcessorCore, GROUP_AFFINITY,
            SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
        },
        Threading::{GetCurrentThread, GetThreadGroupAffinity, SetThreadGroupAffinity},
    },
};

#[derive(Debug, Clone)]
pub struct GroupAffinity {
    pub mask: usize,
    pub group: u16,
}

/// Set thread affinity temporarily, run the closure, restore old affinity
pub fn with_affinity<F, R>(aff: &GroupAffinity, f: F) -> Result<R, String>
where
    F: FnOnce() -> Result<R, String>,
{
    unsafe {
        let thread = GetCurrentThread();

        // Save old affinity
        let mut prev: GROUP_AFFINITY = std::mem::zeroed();
        if !GetThreadGroupAffinity(thread, &mut prev).as_bool() {
            return Err("GetThreadGroupAffinity failed".into());
        }

        // Set new affinity
        let new_aff = GROUP_AFFINITY {
            Mask: aff.mask,
            Group: aff.group,
            Reserved: [0; 3],
        };
        if !SetThreadGroupAffinity(thread, &new_aff, Some(&mut prev)).as_bool() {
            return Err("SetThreadGroupAffinity failed".into());
        }

        // Run the function
        let result = f();

        // Restore old affinity
        if !SetThreadGroupAffinity(thread, &prev, None).as_bool() {
            return Err("SetThreadGroupAffinity failed".into());
        }

        result
    }
}

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
        println!("prev {:?}", prev);

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

pub fn get_all_group_affinities() -> Result<Vec<GroupAffinity>, String> {
    unsafe {
        // First call: get required buffer size
        let mut return_length: u32 = 0;
        if let Err(e) =
            GetLogicalProcessorInformationEx(RelationProcessorCore, None, &mut return_length)
        {
            let raw_win32 = e.code().0 & 0xFFFF; // extract original Win32 error code
            if raw_win32 != ERROR_INSUFFICIENT_BUFFER.0 as i32 {
                return Err(format!("Unexpected error getting buffer size: {:?}", e));
            }
        }

        // Allocate buffer
        let mut buffer = vec![0u8; return_length as usize];

        // Second call: fill buffer
        GetLogicalProcessorInformationEx(
            RelationProcessorCore,
            Some(buffer.as_mut_ptr() as *mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX),
            &mut return_length,
        )
        .map_err(|e| format!("Failed to get processor info: {:?}", e))?;

        let mut offset = 0;
        let mut affinities = Vec::new();

        while offset < return_length as usize {
            let info_ptr =
                buffer.as_ptr().add(offset) as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX;
            let info = &*info_ptr;

            if info.Relationship == RelationProcessorCore {
                let processor = &info.Anonymous.Processor;

                for group_index in 0..processor.GroupCount as usize {
                    let group_affinity = &processor.GroupMask[group_index];
                    let mut mask = group_affinity.Mask;

                    while mask != 0 {
                        let lsb = mask.trailing_zeros();
                        let smask = 1 << lsb;
                        affinities.push(GroupAffinity {
                            mask: smask as usize,
                            group: group_affinity.Group,
                        });
                        mask &= !smask;
                    }
                }
            }

            // println!("info.Size: {}", info.Size);

            offset += info.Size as usize;
        }

        Ok(affinities)
    }
}
