use crate::{
    drivers::{
        driver::Driver,
        pawn_io::pawn_io::{PawnIo, PawnIoError},
    },
    system::cpu::affinity::{error::AffinityError, group::GroupAffinity, utils::with_affinity},
};

#[derive(Debug)]
pub enum AmdFamily0FError {
    /// The PawnIO driver or the specific MSR module isn't loaded/found
    DriverNotLoaded(PawnIoError),
    /// Error during the actual MSR read/write operation
    DriverError(PawnIoError),
    /// Failed to switch or restore thread affinity
    AffinityError(AffinityError),
}

// Convert PawnIoError (from .new() and .execute()) to AmdFamily0FError
impl From<PawnIoError> for AmdFamily0FError {
    fn from(err: PawnIoError) -> Self {
        match err {
            PawnIoError::DriverNotLoaded => Self::DriverNotLoaded(err),
            _ => Self::DriverError(err),
        }
    }
}

// Convert AffinityError to AmdFamily0FError
impl From<AffinityError> for AmdFamily0FError {
    fn from(err: AffinityError) -> Self {
        Self::AffinityError(err)
    }
}

#[derive(Debug)]
pub struct AmdFamily0F {
    pawn_io: PawnIo,
}

impl Driver for AmdFamily0F {
    fn shutdown(&mut self) {
        self.pawn_io.close();
    }
}

impl AmdFamily0F {
    pub fn new() -> Self {
        let pawn_io = PawnIo::load_module_from_file_or_empty("AMDFamily0F.bin");

        Self { pawn_io }
    }

    /// Reads MSR and returns full 64-bit value
    pub fn read_msr(&self, index: u32) -> Result<(u32, u32), AmdFamily0FError> {
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
    ) -> Result<(u32, u32), AmdFamily0FError> {
        with_affinity(affinity, || self.read_msr(index))
    }

    pub fn get_thermtrip(&self, cpu_index: u32, core_index: u32) -> Result<u32, AmdFamily0FError> {
        let mut buffer = 0u64;
        self.pawn_io.execute(
            "ioctl_get_thermtrip",
            Some(&[cpu_index, core_index]),
            Some(&mut buffer),
        )?;

        Ok((buffer & 0xFFFF_FFFF) as u32)
    }

    /// Close underlying PawnIo handle
    pub fn close(&mut self) {
        self.pawn_io.close();
    }
}
