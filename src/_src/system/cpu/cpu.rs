use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::_src::{
    drivers::pawn_io::intel_msr::IntelMsr,
    system::{
        backend::Backend,
        cpu::{
            backends::thread_backend::ThreadBackend,
            intel::{micro_architecture::MicroArchitecture, utils::get_cpu_tjmax_info},
            sensors::{intel::temparature::IntelCpuTempSensor, thread_load::ThreadLoadSensor},
        },
        sensor::Sensor,
        system::System,
    },
};

use super::{
    affinity::group::GroupAffinity,
    core::core::Core,
    core::thread::thread::Thread,
    cpuid::topology::{gather_cpu_affinity_info, CpuAffinityInfo},
    vendor::Vendor,
};

#[derive(Debug)]
pub struct Cpu {
    pub package_id: Option<u32>,
    pub vendor: Vendor,
    pub brand_string: String,
    pub family_id: u8,
    pub model_id: u8,
    pub stepping_id: u8,
    pub cores: Vec<Core>,
    /// Threads that are not assigned to a specific core (used for Unsupported CPUs)
    pub unassigned_threads: Vec<Thread>,
    affinity: GroupAffinity,
}

impl Cpu {
    pub fn new(
        package_id: Option<u32>,
        vendor: Vendor,
        brand_string: String,
        family_id: u8,
        model_id: u8,
        stepping_id: u8,
        affinity: GroupAffinity,
    ) -> Self {
        Self {
            package_id,
            vendor,
            brand_string,
            family_id,
            model_id,
            stepping_id,
            cores: Vec::new(),
            unassigned_threads: Vec::new(),
            affinity,
        }
    }

    pub fn discover(system: &mut System) -> Vec<Cpu> {
        let affinities = gather_cpu_affinity_info().unwrap_or_default();
        if affinities.is_empty() {
            return Vec::new();
        }

        let cores_per_package = Self::calculate_cores_per_package(&affinities);
        let backend = Rc::new(RefCell::new(ThreadBackend::new(affinities.len())));

        let mut cpus: Vec<Cpu> = Vec::new();
        let mut cpu_indices: HashMap<Option<u32>, usize> = HashMap::new();

        for info in affinities {
            match info {
                CpuAffinityInfo::Supported {
                    package_id,
                    core_id,
                    smt_id,
                    affinity,
                    cpuid,
                } => {
                    let cpu_idx = Self::get_or_create_cpu(
                        &mut cpus,
                        &mut cpu_indices,
                        system,
                        &cpuid,
                        Some(package_id),
                        &cores_per_package,
                        &affinity,
                    );

                    let cpu = &mut cpus[cpu_idx];
                    let core = Self::get_or_create_core(
                        cpu, core_id, package_id, system, &cpuid, &affinity,
                    );

                    // Add Thread Load Sensor
                    system.add_sensor(Sensor {
                        id: format!(
                            "/cpu/{}/core/{}/thread/{}/load",
                            package_id, core_id, smt_id
                        ),
                        impl_: Box::new(ThreadLoadSensor::new(&backend, affinity.clone())),
                        parameters: None,
                    });

                    core.threads.push(Thread::new(smt_id, affinity));
                }
                CpuAffinityInfo::Unsupported { affinity, cpuid } => {
                    let cpu_idx = Self::get_or_create_cpu(
                        &mut cpus,
                        &mut cpu_indices,
                        system,
                        &cpuid,
                        None,
                        &cores_per_package,
                        &affinity,
                    );

                    system.add_sensor(Sensor {
                        id: format!("/cpu/?/thread/{}/load", affinity.to_flat_index()),
                        impl_: Box::new(ThreadLoadSensor::new(&backend, affinity.clone())),
                        parameters: None,
                    });

                    cpus[cpu_idx]
                        .unassigned_threads
                        .push(Thread::new(0, affinity));
                }
            }
        }

        system.register_backend(&(backend as Rc<RefCell<dyn Backend>>));
        cpus
    }

    /// Helper: Find existing CPU in vector or initialize a new one with its sensors
    fn get_or_create_cpu(
        cpus: &mut Vec<Cpu>,
        indices: &mut HashMap<Option<u32>, usize>,
        system: &mut System,
        cpuid: &raw_cpuid::CpuId<impl raw_cpuid::CpuIdReader>,
        package_id: Option<u32>,
        cores_per_package: &HashMap<u32, usize>,
        affinity: &GroupAffinity,
    ) -> usize {
        *indices.entry(package_id).or_insert_with(|| {
            let feature_info = cpuid.get_feature_info();
            let family = feature_info.as_ref().map_or(0, |f| f.family_id());
            let model = feature_info.as_ref().map_or(0, |f| f.model_id());
            let stepping = feature_info.as_ref().map_or(0, |f| f.stepping_id());
            let core_count = package_id
                .and_then(|id| cores_per_package.get(&id))
                .copied()
                .unwrap_or(0);

            let vendor = Self::determine_vendor(cpuid, family, model, stepping, core_count);
            let brand = cpuid
                .get_processor_brand_string()
                .map_or(String::new(), |s| s.as_str().to_string());

            if let Some(pkg_id) = package_id {
                Self::try_add_intel_temp_sensor(system, cpuid, pkg_id, None, affinity, &vendor);
            }

            cpus.push(Cpu::new(
                package_id,
                vendor,
                brand,
                family,
                model,
                stepping,
                affinity.clone(),
            ));
            cpus.len() - 1
        })
    }

    /// Helper: Find core in CPU or initialize new one with its core-level sensors
    fn get_or_create_core<'a>(
        cpu: &'a mut Cpu,
        core_id: u32,
        package_id: u32,
        system: &mut System,
        cpuid: &raw_cpuid::CpuId<impl raw_cpuid::CpuIdReader>,
        affinity: &GroupAffinity,
    ) -> &'a mut Core {
        let exists = cpu.cores.iter().any(|c| c.core_id == core_id);

        if !exists {
            Self::try_add_intel_temp_sensor(
                system,
                cpuid,
                package_id,
                Some(core_id),
                affinity,
                &cpu.vendor,
            );
            cpu.cores.push(Core::new(core_id));
        }

        cpu.cores.iter_mut().find(|c| c.core_id == core_id).unwrap()
    }

    /// Common logic for adding Package and Core temperature sensors
    fn try_add_intel_temp_sensor(
        system: &mut System,
        cpuid: &raw_cpuid::CpuId<impl raw_cpuid::CpuIdReader>,
        package_id: u32,
        core_id: Option<u32>,
        affinity: &GroupAffinity,
        vendor: &Vendor,
    ) {
        let Vendor::Intel {
            micro_architecture, ..
        } = vendor
        else {
            return;
        };
        let Some(tp_info) = cpuid.get_thermal_power_info() else {
            return;
        };

        if *micro_architecture != MicroArchitecture::Unknown && tp_info.has_dts() {
            let driver = system
                .get_driver::<IntelMsr>()
                .unwrap_or_else(|| system.insert_driver(IntelMsr::new()));

            let tj_max = get_tj_max_from_msr(&driver, affinity.clone());

            let sensor_id = match core_id {
                Some(id) => format!("/cpu/{}/core/{}/temp", package_id, id),
                None => format!("/cpu/{}/temp", package_id),
            };

            system.add_sensor(Sensor {
                id: sensor_id,
                impl_: Box::new(IntelCpuTempSensor {
                    driver: driver.clone(),
                    affinity: affinity.clone(),
                }),
                parameters: Some(vec![tj_max, 1.0]),
            });
        }
    }

    fn calculate_cores_per_package<R: raw_cpuid::CpuIdReader>(
        affinities: &[CpuAffinityInfo<R>],
    ) -> HashMap<u32, usize> {
        let mut map: HashMap<u32, HashSet<u32>> = HashMap::new();
        for info in affinities {
            if let CpuAffinityInfo::Supported {
                package_id,
                core_id,
                ..
            } = info
            {
                map.entry(*package_id).or_default().insert(*core_id);
            }
        }
        map.into_iter().map(|(k, v)| (k, v.len())).collect()
    }

    /// Helper to determine vendor information from CPUID
    fn determine_vendor<R: raw_cpuid::CpuIdReader>(
        cpuid: &raw_cpuid::CpuId<R>,
        family: u8,
        model: u8,
        stepping: u8,
        core_count: usize,
    ) -> Vendor {
        match cpuid.get_vendor_info() {
            Some(vendor) => match vendor.as_str() {
                "GenuineIntel" => {
                    let (tj_max, micro_architecture) =
                        get_cpu_tjmax_info(family, model, stepping, core_count);
                    Vendor::Intel {
                        micro_architecture,
                        tj_max: tj_max,
                    }
                }
                "AuthenticAMD" | "HygonGenuine" => Vendor::Amd,
                other_name => Vendor::Unknown(Some(other_name.to_string())),
            },
            None => Vendor::Unknown(None),
        }
    }
}

const IA32_TEMPERATURE_TARGET: u32 = 0x01A2;

fn get_tj_max_from_msr(driver: &Rc<RefCell<IntelMsr>>, group_affinity: GroupAffinity) -> f32 {
    let res = driver
        .borrow()
        .read_msr_affinity(IA32_TEMPERATURE_TARGET, &group_affinity);

    match res {
        Ok((eax, _)) => ((eax >> 16) & 0xFF) as f32,
        Err(_) => 100.0,
    }
}
