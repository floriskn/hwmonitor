use std::fmt;

use crate::system::cpu::affinity::error::AffinityError;

#[derive(Debug)]
pub enum CpuidError {
    /// Underlying affinity error
    Affinity(AffinityError),

    /// CPUID instruction not supported / missing data
    MissingCpuidLeaf(&'static str),

    /// CPU vendor not supported
    UnsupportedVendor(String),

    /// Unsupported or unknown topology level
    UnsupportedTopologyLevel,
}

impl fmt::Display for CpuidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Affinity(e) => write!(f, "Affinity error: {}", e),
            Self::MissingCpuidLeaf(s) => write!(f, "Missing CPUID leaf: {}", s),
            Self::UnsupportedVendor(v) => write!(f, "Unsupported CPU vendor: {}", v),
            Self::UnsupportedTopologyLevel => write!(f, "Unsupported topology level"),
        }
    }
}

impl std::error::Error for CpuidError {}
