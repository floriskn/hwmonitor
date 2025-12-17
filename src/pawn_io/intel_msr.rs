use std::sync::Mutex;

use crate::{pawn_io::pawn_io::PawnIo, system::system::Driver};

use windows::Win32::System::{
    SystemInformation::GROUP_AFFINITY,
    Threading::{GetCurrentThread, GetThreadGroupAffinity, SetThreadGroupAffinity},
};

#[derive(Debug)]
pub struct IntelMsr {
    pawn_io: PawnIo,
}

impl Driver for IntelMsr {
    fn shutdown(&mut self) {
        self.pawn_io.close();
    }
}

impl IntelMsr {
    pub fn new() -> Self {
        // Load the IntelMSR.bin resource
        let pawn_io = PawnIo::load_module_from_file("IntelMSR.bin");
        Self { pawn_io }
    }

    /// Reads MSR and returns full 64-bit value
    pub fn read_msr(&self, index: u32) -> Result<(u32, u32), String> {
        // let mut buffer: u64 = 0;
        let mut buffer = 0u64;
        self.pawn_io
            .execute("ioctl_read_msr", Some(&index), Some(&mut buffer))?;

        let eax = (buffer & 0xFFFF_FFFF) as u32;
        let edx = ((buffer >> 32) & 0xFFFF_FFFF) as u32;

        Ok((eax, edx))
    }

    /// Reads MSR with specified thread affinity (placeholder)
    pub fn read_msr_affinity(
        &self,
        index: u32,
        affinity: &GroupAffinity,
    ) -> Result<(u32, u32), String> {
        with_affinity(affinity, || self.read_msr(index))
    }

    /// Close underlying PawnIo handle
    pub fn close(&mut self) {
        self.pawn_io.close();
    }
}

/// Placeholder for GroupAffinity, implement thread affinity if needed
#[derive(Debug, Clone, Copy)]
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
