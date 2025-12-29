use raw_cpuid::{CpuId, CpuIdReader};

use crate::system::cpu::{affinity::group::GroupAffinity, vendor::Vendor};

pub struct CpuNode<R: CpuIdReader> {
    pub package_id: Option<u32>,
    pub vendor: Vendor,
    pub affinity: GroupAffinity,
    pub cores: Vec<CoreNode>,
    pub unassigned_threads: Vec<ThreadNode>,
    pub cpuid: CpuId<R>,
}

pub struct CoreNode {
    pub core_id: u32,
    pub affinity: GroupAffinity,
    pub threads: Vec<ThreadNode>,
}

pub struct ThreadNode {
    pub smt_id: u32,
    pub affinity: GroupAffinity,
}
