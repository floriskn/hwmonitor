use windows::Win32::{
    Foundation::ERROR_INSUFFICIENT_BUFFER,
    System::SystemInformation::{
        GetLogicalProcessorInformationEx, RelationProcessorCore,
        SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
    },
};

use crate::system::cpu::group_affinity::GroupAffinity;

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
                        let single_mask = 1 << lsb;
                        affinities.push(GroupAffinity {
                            mask: single_mask as usize,
                            group: group_affinity.Group,
                        });
                        mask &= !single_mask;
                    }
                }
            }

            // println!("info.Size: {}", info.Size);

            offset += info.Size as usize;
        }

        Ok(affinities)
    }
}
