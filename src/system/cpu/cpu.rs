use std::sync::Arc;

use raw_cpuid::{CpuId, CpuIdReader};

use crate::system::{
    cpu::{
        backend::{amd::AmdBackend, intel::IntelBackend, unknown::UnknownBackend, CpuBackend},
        core::{
            backend::{
                amd::AmdCoreBackend, intel::IntelCoreBackend, unknown::UnknownCoreBackend,
                CoreBackend,
            },
            core::Core,
        },
        group_affinity::{get_all_group_affinities, run_on_all_affinities, GroupAffinity},
        thread::Thread,
        topology::{get_legacy_info, get_topology_info},
        vendor::{get_vendor, Vendor},
    },
    kernal_driver::KernelDriver,
};

pub struct Cpu {
    backend: Arc<dyn CpuBackend + Send + Sync>,
    pub package_id: u32,
    pub vendor: Vendor,
    pub model: String,
    pub cores: Vec<Core>,
    affinity: GroupAffinity,
}

impl Cpu {
    pub fn package_temp(&self) -> Result<f32, String> {
        self.backend.read_package_temp(&self.affinity)
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
            .field("cores", &self.cores)
            .field("affinity", &self.affinity)
            .finish()
    }
}

pub fn gather_cpus(driver: &Arc<KernelDriver>) -> Result<Vec<Cpu>, String> {
    let affinities = get_all_group_affinities()?;
    let mut cpus = Vec::new();

    let results = run_on_all_affinities(affinities, |affinity| detect_cpu(affinity))?;
    for (affinity, info) in results {
        insert_cpu_info(&mut cpus, affinity, info?, driver);
    }

    Ok(cpus)
}

fn detect_cpu(
    affinity: GroupAffinity,
) -> (
    GroupAffinity,
    Result<(u32, u32, u32, Vendor, String), String>,
) {
    let cpuid = CpuId::new();
    let vendor = get_vendor(&cpuid);
    let model = get_model(&cpuid);

    let info = if let Some(topoiter) = cpuid.get_extended_topology_info() {
        get_topology_info(topoiter, vendor, &model)
    } else {
        get_legacy_info(&cpuid, vendor, &model)
    };

    (affinity, info)
}

fn get_model<R: CpuIdReader>(cpuid: &CpuId<R>) -> String {
    cpuid
        .get_processor_brand_string()
        .map(|s| s.as_str().to_string())
        .unwrap_or_default()
}

fn insert_cpu_info(
    cpus: &mut Vec<Cpu>,
    affinity: GroupAffinity,
    info: (u32, u32, u32, Vendor, String),
    driver: &Arc<KernelDriver>,
) {
    let (package_id, core_id, smt_id, vendor, model) = info;

    if let Some(cpu) = cpus.iter_mut().find(|c| c.package_id == package_id) {
        if is_lower_affinity(&affinity, &cpu.affinity) {
            cpu.affinity = affinity.clone();
        }

        if let Some(core) = cpu.cores.iter_mut().find(|c| c.core_id == core_id) {
            core.threads
                .push(Thread::new(smt_id, affinity, cpu.backend.clone()));
        } else {
            let backend: Arc<dyn CoreBackend + Send + Sync> = match vendor {
                Vendor::Intel => Arc::new(IntelCoreBackend::new(driver.clone(), affinity.clone())),
                Vendor::Amd => Arc::new(AmdCoreBackend::new(driver.clone())),
                Vendor::Unknown(_) => Arc::new(UnknownCoreBackend::new(driver.clone())),
            };
            let mut core = Core::new(core_id, backend);
            core.threads
                .push(Thread::new(smt_id, affinity, cpu.backend.clone()));
            cpu.cores.push(core);
        }
    } else {
        let backend: Arc<dyn CpuBackend + Send + Sync> = match vendor {
            Vendor::Intel => Arc::new(IntelBackend::new(driver.clone())),
            Vendor::Amd => Arc::new(AmdBackend::new(driver.clone())),
            Vendor::Unknown(_) => Arc::new(UnknownBackend::new(driver.clone())),
        };
        let core_backend: Arc<dyn CoreBackend + Send + Sync> = match vendor {
            Vendor::Intel => Arc::new(IntelCoreBackend::new(driver.clone(), affinity.clone())),
            Vendor::Amd => Arc::new(AmdCoreBackend::new(driver.clone())),
            Vendor::Unknown(_) => Arc::new(UnknownCoreBackend::new(driver.clone())),
        };

        let mut core = Core::new(core_id, core_backend);
        core.threads
            .push(Thread::new(smt_id, affinity.clone(), backend.clone()));

        cpus.push(Cpu {
            backend,
            package_id,
            vendor,
            model,
            affinity,
            cores: vec![core],
        });
    }
}

fn is_lower_affinity(a: &GroupAffinity, b: &GroupAffinity) -> bool {
    (a.group < b.group) || (a.group == b.group && a.mask < b.mask)
}
