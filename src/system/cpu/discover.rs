use std::collections::{HashMap, HashSet};

use raw_cpuid::CpuIdReaderNative;

use super::{
    affinity::group::GroupAffinity,
    cpuid::topology::{gather_cpu_affinity_info, CpuAffinityInfo},
    topology::{CoreNode, CpuNode, ThreadNode},
    vendor::Vendor,
};

/// Discover the CPU topology and build a vendor-agnostic CPU tree.
///
/// This function is intentionally **side-effect free**:
/// - No sensors are registered
/// - No backends are created
/// - No drivers are touched
///
/// Its sole responsibility is to translate CPUID + affinity information
/// into a stable logical model:
///
///     CPU (package) → cores → threads
///
/// Unsupported CPUs are grouped under a synthetic CPU node
/// with `package_id == None`.
pub fn discover_topology() -> (Vec<CpuNode<CpuIdReaderNative>>, usize) {
    let affinities = gather_cpu_affinity_info().unwrap_or_default();
    if affinities.is_empty() {
        return (Vec::new(), 0);
    }

    // Resulting CPU tree
    let mut cpus: Vec<CpuNode<CpuIdReaderNative>> = Vec::new();

    // Maps a physical package_id to an index in `cpus`
    //
    // NOTE:
    // - `Some(id)` → real physical CPU package
    // - `None`     → unsupported / unknown CPU
    let mut cpu_indices: HashMap<Option<u32>, usize> = HashMap::new();

    let threads = affinities.len();

    for info in affinities {
        match info {
            CpuAffinityInfo::Supported {
                package_id,
                core_id,
                smt_id,
                affinity,
                cpuid,
            } => {
                let cpu_idx = get_or_create_cpu(
                    &mut cpus,
                    &mut cpu_indices,
                    &cpuid,
                    Some(package_id),
                    &affinity,
                );

                let cpu = &mut cpus[cpu_idx];
                let core = get_or_create_core(cpu, core_id);

                core.threads.push(ThreadNode { smt_id, affinity });
            }
            CpuAffinityInfo::Unsupported { affinity, cpuid } => {
                // Unsupported CPUs still get a logical CPU node so
                // thread-level sensors can be attached consistently.
                let cpu_idx =
                    get_or_create_cpu(&mut cpus, &mut cpu_indices, &cpuid, None, &affinity);

                cpus[cpu_idx].unassigned_threads.push(ThreadNode {
                    // SMT id is meaningless here; keep it deterministic.
                    smt_id: 0,
                    affinity,
                });
            }
        }
    }

    (cpus, threads)
}

/// Find an existing CPU node or create a new one.
///
/// Invariants:
/// - Exactly one `CpuNode` exists per `package_id`
/// - Vendor detection happens **once per CPU**
fn get_or_create_cpu(
    cpus: &mut Vec<CpuNode<CpuIdReaderNative>>,
    indices: &mut HashMap<Option<u32>, usize>,
    cpuid: &raw_cpuid::CpuId<CpuIdReaderNative>,
    package_id: Option<u32>,
    affinity: &GroupAffinity,
) -> usize {
    *indices.entry(package_id).or_insert_with(|| {
        let vendor = determine_vendor(cpuid);

        cpus.push(CpuNode {
            cpuid: *cpuid,
            package_id,
            vendor,
            // NOTE:
            // This affinity is representative for the package and is
            // primarily used for package-level backends/sensors.
            affinity: affinity.clone(),
            cores: Vec::new(),
            unassigned_threads: Vec::new(),
        });
        cpus.len() - 1
    })
}

/// Find an existing core in a CPU or create it.
///
/// Invariant:
/// - Core IDs are unique per CPU package
fn get_or_create_core(cpu: &mut CpuNode<CpuIdReaderNative>, core_id: u32) -> &mut CoreNode {
    // Phase 1: find index (immutable borrow)
    if let Some(idx) = cpu.cores.iter().position(|c| c.core_id == core_id) {
        return &mut cpu.cores[idx];
    }

    // Phase 2: mutate
    cpu.cores.push(CoreNode {
        core_id,
        threads: Vec::new(),
    });

    cpu.cores.last_mut().unwrap()
}

/// Determine the CPU vendor based on CPUID.
///
/// IMPORTANT:
/// This function must stay **cheap and side-effect free**.
/// Detailed vendor probing belongs in the vendor implementation,
/// not during topology discovery.
fn determine_vendor<R: raw_cpuid::CpuIdReader>(cpuid: &raw_cpuid::CpuId<R>) -> Vendor {
    match cpuid.get_vendor_info() {
        Some(vendor) => match vendor.as_str() {
            "GenuineIntel" => Vendor::Intel,
            "AuthenticAMD" | "HygonGenuine" => Vendor::Amd,
            other_name => Vendor::Unknown(Some(other_name.to_string())),
        },
        None => Vendor::Unknown(None),
    }
}
