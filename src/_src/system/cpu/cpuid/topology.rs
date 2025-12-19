// (Cpuid, Affinity, Id: u32)

use raw_cpuid::{
    CpuId, CpuIdReader, CpuIdReaderNative, ExtendedTopologyLevel, TopologyType, VendorInfo,
};

use crate::_src::system::cpu::affinity::{
    group::{get_all_group_affinities, GroupAffinity},
    utils::run_on_all_affinities,
};

use super::{error::CpuidError, utils::cpuid_bits_needed};

#[derive(Debug)]
pub enum CpuAffinityInfo<R: CpuIdReader> {
    /// Topology extraction succeeded
    Supported {
        affinity: GroupAffinity,
        package_id: u32,
        core_id: u32,
        smt_id: u32,
        cpuid: CpuId<R>,
    },
    /// Topology extraction failed
    Unsupported {
        affinity: GroupAffinity,
        cpuid: CpuId<R>,
    },
}

pub fn gather_cpu_affinity_info() -> Result<Vec<CpuAffinityInfo<CpuIdReaderNative>>, CpuidError> {
    // Step 1: Get all affinities
    let affinities = get_all_group_affinities().map_err(CpuidError::Affinity)?;

    // Step 2: Gather CPU info, capturing errors in the Result
    let mut infos: Vec<CpuAffinityInfo<CpuIdReaderNative>> =
        run_on_all_affinities(affinities, |affinity| {
            let cpuid = CpuId::new();

            // Attempt to get IDs from extended topology or standard xapic
            let ids = if let Some(topoiter) = cpuid.get_extended_topology_info() {
                extract_x2apic_ids(topoiter)
            } else {
                extract_xapic_ids(&cpuid)
            };

            match ids {
                Ok((package_id, core_id, smt_id)) => CpuAffinityInfo::Supported {
                    affinity,
                    package_id,
                    core_id,
                    smt_id,
                    cpuid,
                },
                Err(_) => CpuAffinityInfo::Unsupported { affinity, cpuid },
            }
        })
        .map_err(CpuidError::Affinity)?;

    // Step 3: Sort
    // Supported items are compared by IDs; Unsupported items are moved to the end.
    infos.sort_by(|a, b| match (a, b) {
        // Both are Supported: Sort by hierarchical hardware IDs
        (
            CpuAffinityInfo::Supported {
                package_id: ap,
                core_id: ac,
                smt_id: asmt,
                ..
            },
            CpuAffinityInfo::Supported {
                package_id: bp,
                core_id: bc,
                smt_id: bsmt,
                ..
            },
        ) => ap.cmp(bp).then(ac.cmp(bc)).then(asmt.cmp(bsmt)),

        // Supported always comes before Unsupported
        (CpuAffinityInfo::Supported { .. }, CpuAffinityInfo::Unsupported { .. }) => {
            std::cmp::Ordering::Less
        }
        (CpuAffinityInfo::Unsupported { .. }, CpuAffinityInfo::Supported { .. }) => {
            std::cmp::Ordering::Greater
        }

        // Both are Unsupported: Sort by the flat processor index
        (
            CpuAffinityInfo::Unsupported {
                affinity: affinity_a,
                ..
            },
            CpuAffinityInfo::Unsupported {
                affinity: affinity_b,
                ..
            },
        ) => affinity_a.to_flat_index().cmp(&affinity_b.to_flat_index()),
    });

    Ok(infos)
}

fn extract_x2apic_ids(
    topoiter: impl Iterator<Item = ExtendedTopologyLevel>,
) -> Result<(u32, u32, u32), CpuidError> {
    let topology: Vec<ExtendedTopologyLevel> = topoiter.collect();

    let mut smt_found = false;
    let mut core_found = false;

    let mut smt_x2apic_shift: u32 = 0;
    let mut core_x2apic_shift: u32 = 0;

    for level in &topology {
        match level.level_type() {
            TopologyType::SMT => {
                smt_x2apic_shift = level.shift_right_for_next_apic_id();
                smt_found = true;
            }
            TopologyType::Core => {
                core_x2apic_shift = level.shift_right_for_next_apic_id();
                core_found = true;
            }
            _ => return Err(CpuidError::UnsupportedTopologyLevel),
        }

        // Early exit once both types have been found
        if smt_found && core_found {
            break;
        }
    }

    // Techinacly cannot fail
    let x2apic_id = topology.first().map(|l| l.x2apic_id()).unwrap_or(0);

    let smt_select_mask = !(u32::max_value() << smt_x2apic_shift);
    let core_select_mask = (!((u32::max_value()) << core_x2apic_shift)) ^ smt_select_mask;
    let pkg_select_mask = u32::max_value() << core_x2apic_shift;

    let smt_id = x2apic_id & smt_select_mask;
    let core_id = (x2apic_id & core_select_mask) >> smt_x2apic_shift;
    let pkg_id = (x2apic_id & pkg_select_mask) >> core_x2apic_shift;

    Ok((pkg_id, core_id, smt_id))
}

fn extract_xapic_ids<R: CpuIdReader>(cpuid: &CpuId<R>) -> Result<(u32, u32, u32), CpuidError> {
    let (max_logical_processor_ids, smt_max_cores_for_package) = get_processor_limits(&cpuid)?;

    let smt_mask_width: u8 = cpuid_bits_needed(
        (max_logical_processor_ids.next_power_of_two() / smt_max_cores_for_package) - 1,
    );
    let smt_select_mask: u8 = !(u8::max_value() << smt_mask_width);
    let core_mask_width: u8 = cpuid_bits_needed(smt_max_cores_for_package - 1);
    let core_only_select_mask =
        (!(u8::max_value() << (core_mask_width + smt_mask_width))) ^ smt_select_mask;
    let pkg_select_mask = u8::max_value() << (core_mask_width + smt_mask_width);

    let xapic_id = cpuid
        .get_feature_info()
        .map_or_else(|| 0, |finfo| finfo.initial_local_apic_id());

    let smt_id = xapic_id & smt_select_mask;
    let core_id = (xapic_id & core_only_select_mask) >> smt_mask_width;
    let pkg_id = (xapic_id & pkg_select_mask) >> (core_mask_width + smt_mask_width);

    Ok((pkg_id as u32, core_id as u32, smt_id as u32))
}

fn get_processor_limits<R: CpuIdReader>(cpuid: &CpuId<R>) -> Result<(u8, u8), CpuidError> {
    let vendor_info = cpuid.get_vendor_info();
    let vendor_name = vendor_info
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("Unknown");

    match vendor_name {
        "GenuineIntel" => get_intel_limits(cpuid),
        "AuthenticAMD" | "HygonGenuine" => get_amd_limits(cpuid),
        _ => {
            // Last resort: Try AMD-style capacity info
            if let Ok(limits) = get_amd_limits(cpuid) {
                if limits.0 > 0 && limits.1 > 0 {
                    return Ok(limits);
                }
            }

            // Last resort: Try Intel-style cache parameters
            if let Ok(limits) = get_intel_limits(cpuid) {
                if limits.0 > 0 && limits.1 > 0 {
                    return Ok(limits);
                }
            }

            Err(CpuidError::UnsupportedVendor(vendor_name.to_string()))
        }
    }
}

/// Internal helper for Intel-style limit detection
fn get_intel_limits<R: CpuIdReader>(cpuid: &CpuId<R>) -> Result<(u8, u8), CpuidError> {
    let max_logical_processor_identifiers = cpuid
        .get_feature_info()
        .map(|feature| feature.max_logical_processor_ids())
        .ok_or(CpuidError::MissingCpuidLeaf("feature_info"))?;

    let cache_parameters = cpuid
        .get_cache_parameters()
        .ok_or(CpuidError::MissingCpuidLeaf("cache_parameters"))?;

    let mut smt_maximum_cores_for_package: u8 = 1;
    for (index, cache) in cache_parameters.enumerate() {
        if index == 0 {
            smt_maximum_cores_for_package = cache.max_cores_for_package() as u8;
            break;
        }
    }

    Ok((
        max_logical_processor_identifiers,
        smt_maximum_cores_for_package,
    ))
}

/// Internal helper for AMD-style limit detection
fn get_amd_limits<R: CpuIdReader>(cpuid: &CpuId<R>) -> Result<(u8, u8), CpuidError> {
    let capacity_info =
        cpuid
            .get_processor_capacity_feature_info()
            .ok_or(CpuidError::MissingCpuidLeaf(
                "processor_capacity_feature_info",
            ))?;

    Ok((
        capacity_info.num_phys_threads() as u8,
        capacity_info.apic_id_size(),
    ))
}
