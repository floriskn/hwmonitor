use std::{sync::Arc, thread};

use core_affinity;
use raw_cpuid::{CpuId, CpuIdReader, ExtendedTopologyLevel, TopologyType};

use crate::system::{
    cpu::backend::{amd::AmdBackend, intel::IntelBackend, unknown::UnknownBackend, CpuBackend},
    kernal_driver::KernelDriver,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vendor {
    Intel,
    Amd,
    Unknown(Option<String>),
}

pub struct Thread {
    pub thread_id: u32,
    backend: Arc<dyn CpuBackend + Send + Sync>,
}

pub struct Core {
    backend: Arc<dyn CpuBackend + Send + Sync>,
    pub core_id: u32,
    pub affinity_id: usize,
    pub threads: Vec<Thread>,
}

impl Core {
    pub fn temperature(&self) -> Option<f32> {
        self.backend.read_core_temp(self.core_id)
    }
}

pub struct Cpu {
    backend: Arc<dyn CpuBackend + Send + Sync>,
    pub package_id: u32,
    pub vendor: Vendor,
    pub model: String,
    pub cores: Vec<Core>,
}

impl Cpu {
    pub fn package_temp(&self) -> Result<f32, String> {
        self.backend.read_package_temp(self.package_id)
    }

    pub fn cores(&self) -> &[Core] {
        &self.cores
    }
}

impl std::fmt::Debug for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cpu")
            .field("package_id", &self.package_id)
            .field("vendor", &self.vendor)
            .field("model", &self.model)
            // .field("cores", &self.cores)
            .finish()
    }
}

fn cpuid_bits_needed(count: u8) -> u8 {
    let mut mask: u8 = 0x80;
    let mut cnt: u8 = 8;

    while (cnt > 0) && ((mask & count) != mask) {
        mask >>= 1;
        cnt -= 1;
    }

    cnt
}

pub fn gather_cpus(driver: &Arc<KernelDriver>) -> Result<Vec<Cpu>, String> {
    let core_ids = core_affinity::get_core_ids().unwrap();
    let mut cpus: Vec<Cpu> = Vec::new();

    for core in core_ids {
        let core_clone = core.clone();

        let info = thread::spawn(move || detect_cpu(core_clone))
            .join()
            .map_err(|_| "Thread panicked while detecting CPU".to_string())??;

        insert_cpu_info(&mut cpus, core.id, info, driver);
    }

    Ok(cpus)
}

// Detect CPU topology and info for a single core
fn detect_cpu(core: core_affinity::CoreId) -> Result<(u32, u32, u32, Vendor, String), String> {
    core_affinity::set_for_current(core);

    let cpuid = CpuId::new();

    let vendor = get_vendor(&cpuid);
    let model = get_model(&cpuid);

    if let Some(topoiter) = cpuid.get_extended_topology_info() {
        get_topology_info(topoiter, vendor, &model)
    } else {
        get_legacy_info(&cpuid, vendor, &model)
    }
}

// Extract vendor from CPUID
fn get_vendor<R: CpuIdReader>(cpuid: &CpuId<R>) -> Vendor {
    cpuid
        .get_vendor_info()
        .map(|v| match v.as_str() {
            "GenuineIntel" => Vendor::Intel,
            "AuthenticAMD" | "HygonGenuine" => Vendor::Amd,
            name => Vendor::Unknown(Some(name.to_owned())),
        })
        .unwrap_or(Vendor::Unknown(None))
}

// Extract model string from CPUID
fn get_model<R: CpuIdReader>(cpuid: &CpuId<R>) -> String {
    cpuid
        .get_processor_brand_string()
        .map(|s| s.as_str().to_string())
        .unwrap_or_default()
}

// Handle extended topology (x2APIC)
fn get_topology_info(
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

    println!(
        "x2APIC#{} (pkg: {}, core: {}, smt: {})",
        x2apic_id, pkg_id, core_id, smt_id
    );

    Ok((pkg_id, core_id, smt_id, vendor, model.to_string()))
}

// Handle legacy APIC topology
fn get_legacy_info<R: CpuIdReader>(
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

    println!(
        "APIC#{} (pkg: {}, core: {}, smt: {})",
        xapic_id, pkg_id, core_id, smt_id
    );

    Ok((
        pkg_id as u32,
        core_id as u32,
        smt_id as u32,
        vendor,
        model.to_string(),
    ))
}

// Insert CPU info into the main vector
fn insert_cpu_info(
    cpus: &mut Vec<Cpu>,
    affinity_id: usize,
    info: (u32, u32, u32, Vendor, String),
    driver: &Arc<KernelDriver>,
) {
    let (pkg_id, core_id, smt_id, vendor, model) = info;

    if let Some(cpu) = cpus.iter_mut().find(|c| c.package_id == pkg_id) {
        // find an existing Core inside the CPU
        if let Some(core_entry) = cpu.cores.iter_mut().find(|c| c.core_id == core_id) {
            // add a new Thread to an existing Core
            core_entry.threads.push(Thread {
                backend: cpu.backend.clone(),
                thread_id: smt_id,
            });
        } else {
            // create a new Core with its first Thread
            cpu.cores.push(Core {
                backend: cpu.backend.clone(),
                core_id,
                affinity_id,
                threads: vec![Thread {
                    backend: cpu.backend.clone(),
                    thread_id: smt_id,
                }],
            });
        }
    } else {
        let backend: Arc<dyn CpuBackend + Send + Sync> = match vendor {
            Vendor::Intel => Arc::new(IntelBackend::new(driver.clone())),
            Vendor::Amd => Arc::new(AmdBackend::new(driver.clone())),
            Vendor::Unknown(_) => Arc::new(UnknownBackend::new(driver.clone())),
        };

        cpus.push(Cpu {
            backend: backend.clone(),
            package_id: pkg_id,
            vendor,
            model,
            cores: vec![Core {
                backend: backend.clone(),
                core_id,
                affinity_id,
                threads: vec![Thread {
                    backend: backend.clone(),
                    thread_id: smt_id,
                }],
            }],
        });
    }
}
