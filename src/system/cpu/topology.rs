use raw_cpuid::{CpuId, CpuIdReader, ExtendedTopologyLevel, TopologyType};

use crate::system::cpu::vendor::Vendor;

fn cpuid_bits_needed(count: u8) -> u8 {
    let mut mask: u8 = 0x80;
    let mut cnt: u8 = 8;

    while (cnt > 0) && ((mask & count) != mask) {
        mask >>= 1;
        cnt -= 1;
    }

    cnt
}

// Handle extended topology (x2APIC)
pub fn get_topology_info(
    topoiter: impl Iterator<Item = ExtendedTopologyLevel>,
    vendor: Vendor,
    model: &str,
) -> Result<(u32, u32, u32, Vendor, String), String> {
    let topology: Vec<ExtendedTopologyLevel> = topoiter.collect();

    let mut smt_x2apic_shift = 0;
    let mut core_x2apic_shift = 0;

    for level in &topology {
        match level.level_type() {
            TopologyType::SMT => smt_x2apic_shift = level.shift_right_for_next_apic_id(),
            TopologyType::Core => core_x2apic_shift = level.shift_right_for_next_apic_id(),
            _ => return Err("Unsupported topology level type".to_string()),
        }
    }

    // Use the first element of the topology vector for x2apic_id
    let x2apic_id = topology.first().map(|l| l.x2apic_id()).unwrap_or(0);

    let smt_select_mask = !(u32::max_value() << smt_x2apic_shift);
    let core_select_mask = (!((u32::max_value()) << core_x2apic_shift)) ^ smt_select_mask;
    let pkg_select_mask = u32::max_value() << core_x2apic_shift;

    let smt_id = x2apic_id & smt_select_mask;
    let core_id = (x2apic_id & core_select_mask) >> smt_x2apic_shift;
    let pkg_id = (x2apic_id & pkg_select_mask) >> core_x2apic_shift;

    // println!(
    //     "x2APIC#{} (pkg: {}, core: {}, smt: {})",
    //     x2apic_id, pkg_id, core_id, smt_id
    // );

    Ok((pkg_id, core_id, smt_id, vendor, model.to_string()))
}

// Handle legacy APIC topology
pub fn get_legacy_info<R: CpuIdReader>(
    cpuid: &CpuId<R>,
    vendor: Vendor,
    model: &str,
) -> Result<(u32, u32, u32, Vendor, String), String> {
    let (max_logical_processor_ids, smt_max_cores_for_package) = match vendor {
        Vendor::Intel => {
            let cparams = cpuid
                .get_cache_parameters()
                .ok_or("Intel CPU: missing cache parameters")?;
            let max_logical = cpuid
                .get_feature_info()
                .map(|f| f.max_logical_processor_ids())
                .unwrap_or(1);
            let smt_cores = cparams
                .into_iter()
                .next()
                .ok_or("Intel CPU: no cache parameter entries")?
                .max_cores_for_package() as u8;
            (max_logical as u8, smt_cores)
        }
        Vendor::Amd => {
            let info = cpuid
                .get_processor_capacity_feature_info()
                .ok_or("AMD CPU: missing processor capacity info")?;
            (info.num_phys_threads() as u8, info.apic_id_size() as u8)
        }
        Vendor::Unknown(_) => return Err("Unsupported CPU vendor".to_string()),
    };

    let smt_mask_width = cpuid_bits_needed(
        (max_logical_processor_ids.next_power_of_two() / smt_max_cores_for_package) - 1,
    );
    let smt_select_mask = !(u8::max_value() << smt_mask_width);
    let core_mask_width = cpuid_bits_needed(smt_max_cores_for_package - 1);
    let core_only_select_mask =
        (!(u8::max_value() << (core_mask_width + smt_mask_width))) ^ smt_select_mask;
    let pkg_select_mask = u8::max_value() << (core_mask_width + smt_mask_width);

    let xapic_id = cpuid
        .get_feature_info()
        .map_or(0, |f| f.initial_local_apic_id());

    let smt_id = xapic_id & smt_select_mask;
    let core_id = (xapic_id & core_only_select_mask) >> smt_mask_width;
    let pkg_id = (xapic_id & pkg_select_mask) >> (core_mask_width + smt_mask_width);

    // println!(
    //     "APIC#{} (pkg: {}, core: {}, smt: {})",
    //     xapic_id, pkg_id, core_id, smt_id
    // );

    Ok((
        pkg_id as u32,
        core_id as u32,
        smt_id as u32,
        vendor,
        model.to_string(),
    ))
}
