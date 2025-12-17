use raw_cpuid::{CpuId, CpuIdReader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vendor {
    Intel,
    Amd,
    Unknown(Option<String>),
}

// Extract vendor from CPUID
pub fn get_vendor<R: CpuIdReader>(cpuid: &CpuId<R>) -> Vendor {
    cpuid
        .get_vendor_info()
        .map_or(Vendor::Unknown(None), |v| match v.as_str() {
            "GenuineIntel" => Vendor::Intel,
            "AuthenticAMD" | "HygonGenuine" => Vendor::Amd,
            name => Vendor::Unknown(Some(name.to_string())),
        })
}
