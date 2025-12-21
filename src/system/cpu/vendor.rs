use raw_cpuid::{CpuId, CpuIdReader};

use super::intel::{micro_architecture::MicroArchitecture, tj_max::CpuTJMax};

#[derive(Debug, Clone)]
pub enum Vendor {
    Intel {
        time_stamp_counter_multiplier: f64,
        tj_max: CpuTJMax,
        micro_architecture: MicroArchitecture,
    },
    Amd,
    Unknown(Option<String>),
}

// Extract vendor from CPUID
// pub fn get_vendor<R: CpuIdReader>(cpuid: &CpuId<R>) -> Vendor {
//     cpuid
//         .get_vendor_info()
//         .map_or(Vendor::Unknown(None), |v| match v.as_str() {
//             "GenuineIntel" => Vendor::Intel,
//             "AuthenticAMD" | "HygonGenuine" => Vendor::Amd,
//             name => Vendor::Unknown(Some(name.to_string())),
//         })
// }
