use windows::Win32::System::{
    SystemInformation::GROUP_AFFINITY,
    Threading::{GetCurrentThread, GetThreadGroupAffinity, SetThreadGroupAffinity},
};

pub struct GroupAffinity {
    pub mask: usize,
    pub group: u16,
}

/// Set thread affinity temporarily, run the closure, restore old affinity
pub fn with_affinity<F, R>(aff: GroupAffinity, f: F) -> Result<R, String>
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
        if !SetThreadGroupAffinity(thread, &prev, Some(std::ptr::null_mut())).as_bool() {
            return Err("SetThreadGroupAffinity failed".into());
        }

        result
    }
}
