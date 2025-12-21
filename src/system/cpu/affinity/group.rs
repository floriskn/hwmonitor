use std::mem::MaybeUninit;

use windows::Win32::{
    Foundation::ERROR_INSUFFICIENT_BUFFER,
    System::SystemInformation::{
        GetLogicalProcessorInformationEx, RelationProcessorCore,
        SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
    },
};

use super::error::AffinityError;

#[derive(Debug, Clone, Copy)]
pub struct GroupAffinity {
    pub mask: usize,
    pub group: u16,
}

impl GroupAffinity {
    /// Converts a GroupAffinity (group + bitmask) into a single flat index.
    /// This represents the logical processor number across all processor groups.
    pub fn to_flat_index(&self) -> usize {
        #[cfg(target_pointer_width = "64")]
        const MAXIMUM_PROCESSORS_PER_GROUP: usize = 64;

        #[cfg(target_pointer_width = "32")]
        const MAXIMUM_PROCESSORS_PER_GROUP: usize = 32;

        // The bit index is the position of the first set bit in the affinity mask.
        // For example, a mask of 0x4 (0100) has 2 trailing zeros, so bit_index is 2.
        let bit_index = self.mask.trailing_zeros() as usize;

        // flat_index = (group_number * processors_per_group) + specific_bit_within_group
        (self.group as usize * MAXIMUM_PROCESSORS_PER_GROUP) + bit_index
    }
}

pub fn get_all_group_affinities() -> Result<Vec<GroupAffinity>, AffinityError> {
    unsafe {
        // First call: get required buffer size
        let mut return_length: u32 = 0;
        if let Err(e) =
            GetLogicalProcessorInformationEx(RelationProcessorCore, None, &mut return_length)
        {
            let raw_win32 = e.code().0 & 0xFFFF; // extract original Win32 error code
            if raw_win32 != ERROR_INSUFFICIENT_BUFFER.0 as i32 {
                return Err(AffinityError::WinApi {
                    context: "GetLogicalProcessorInformationEx(size)",
                    source: e,
                });
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
        .map_err(|e| AffinityError::WinApi {
            context: "GetLogicalProcessorInformationEx(data)",
            source: e,
        })?;

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
                        let single_mask = 1 << lsb;
                        affinities.push(GroupAffinity {
                            mask: single_mask as usize,
                            group: group_affinity.Group,
                        });
                        mask &= !single_mask;
                    }
                }
            }

            offset += info.Size as usize;
        }

        Ok(affinities)
    }
}
