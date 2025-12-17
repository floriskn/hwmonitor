use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    thread,
    time::Duration,
};

use raw_cpuid::{CpuId, CpuIdReader, FeatureInfo};
use windows::Win32::{
    Foundation::FILETIME,
    System::Threading::{GetCurrentThread, GetThreadTimes},
};
use x86::msr::IA32_PACKAGE_THERM_STATUS;

use crate::{
    pawn_io::intel_msr::{GroupAffinity, IntelMsr},
    system::{
        cpu::{
            core::{Core, CpuLoadBackend},
            group_affinity::{system::get_all_group_affinities, thread::run_on_all_affinities},
            thread::{Thread, ThreadLoadSensor},
            topology::{get_legacy_info, get_topology_info},
            vendor::{get_vendor, Vendor},
        },
        sensor::sensor::{Sensor, SensorImpl, SensorKind, SensorTarget},
        system::{Backend, System},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MicroArchitecture {
    Airmont,
    AlderLake,
    Atom,
    ArrowLake, // Gen 15 (0xC6, -H = 0xC5)
    Broadwell,
    CannonLake,
    CometLake,
    Core,
    Goldmont,
    GoldmontPlus,
    Haswell,
    IceLake,
    IvyBridge,
    JasperLake,
    KabyLake,
    LunarLake,
    Nehalem,
    NetBurst,
    MeteorLake,
    RocketLake,
    SandyBridge,
    Silvermont,
    Skylake,
    TigerLake,
    Tremont,
    RaptorLake,
    SapphireRapids,
    ElkhartLake,
    Unknown,
}

#[derive(Debug)]
enum CpuTJMax {
    MSR,
    Static(f32),
}

#[derive(Debug)]
pub struct Cpu {
    pub package_id: u32,
    pub vendor: Vendor,
    pub model: String,
    pub family_id: u8,
    pub model_id: u8,
    pub stepping_id: u8,
    pub cores: Vec<Core>,
    affinity: GroupAffinity,
    tj_max: CpuTJMax,
    micro_architecture: MicroArchitecture,
}

// TODO: impl gte lte etc
fn is_lower_affinity(a: &GroupAffinity, b: &GroupAffinity) -> bool {
    (a.group, a.mask) < (b.group, b.mask)
}
fn get_tj_max(
    family_id: u8,
    model_id: u8,
    stepping_id: u8,
    core_count: usize,
) -> (CpuTJMax, MicroArchitecture) {
    match family_id {
        0x06 => match model_id {
            0x0F => {
                // Intel Core 2 (65nm)
                let tj = match stepping_id {
                    // B2
                    0x06 => match core_count {
                        2 => CpuTJMax::Static(90.0),
                        4 => CpuTJMax::Static(100.0),
                        _ => CpuTJMax::Static(95.0),
                    },
                    // G0
                    0x0B => CpuTJMax::Static(100.0),
                    // M0
                    0x0D => CpuTJMax::Static(95.0),
                    _ => CpuTJMax::Static(95.0),
                };
                (tj, MicroArchitecture::Core)
            }

            // Intel Core 2 (45nm)
            0x17 => (CpuTJMax::Static(100.0), MicroArchitecture::Core),

            0x1C => {
                // Intel Atom (45nm)
                let tj = match stepping_id {
                    // C0
                    0x02 => CpuTJMax::Static(90.0),
                    // A0, B0
                    0x0A => CpuTJMax::Static(100.0),
                    _ => CpuTJMax::Static(90.0),
                };
                (tj, MicroArchitecture::Atom)
            }

            /* Nehalem
             * 0x1A: Core i7 LGA1366 (45nm)
             * 0x1E: Core i5/i7 LGA1156 (45nm)
             * 0x1F: Core i5/i7
             * 0x25: Core i3/i5/i7 LGA1156 (32nm)
             * 0x2C: Core i7 LGA1366 6C (32nm)
             * 0x2E: Xeon 7500 (45nm)
             * 0x2F: Xeon (32nm)
             */
            0x1A | 0x1E | 0x1F | 0x25 | 0x2C | 0x2E | 0x2F => {
                (CpuTJMax::MSR, MicroArchitecture::Nehalem)
            }

            /* Sandy Bridge */
            0x2A | 0x2D => (CpuTJMax::MSR, MicroArchitecture::SandyBridge),

            /* Ivy Bridge */
            0x3A | 0x3E => (CpuTJMax::MSR, MicroArchitecture::IvyBridge),

            /* Haswell */
            0x3C | 0x3F | 0x45 | 0x46 => (CpuTJMax::MSR, MicroArchitecture::Haswell),

            /* Broadwell */
            0x3D | 0x47 | 0x4F | 0x56 => (CpuTJMax::MSR, MicroArchitecture::Broadwell),

            /* Atom */
            0x36 => (CpuTJMax::MSR, MicroArchitecture::Atom),

            /* Silvermont */
            0x37 | 0x4A | 0x4D | 0x5A | 0x5D => (CpuTJMax::MSR, MicroArchitecture::Silvermont),

            /* Skylake */
            0x4E | 0x5E | 0x55 => (CpuTJMax::MSR, MicroArchitecture::Skylake),

            /* Airmont */
            0x4C => (CpuTJMax::MSR, MicroArchitecture::Airmont),

            /* Kaby / Coffee Lake */
            0x8E | 0x9E => (CpuTJMax::MSR, MicroArchitecture::KabyLake),

            /* Goldmont */
            0x5C | 0x5F => (CpuTJMax::MSR, MicroArchitecture::Goldmont),

            /* Goldmont Plus */
            0x7A => (CpuTJMax::MSR, MicroArchitecture::GoldmontPlus),

            /* Cannon Lake */
            0x66 => (CpuTJMax::MSR, MicroArchitecture::CannonLake),

            /* Ice Lake */
            0x7D | 0x7E | 0x6A | 0x6C => (CpuTJMax::MSR, MicroArchitecture::IceLake),

            /* Comet Lake */
            0xA5 | 0xA6 => (CpuTJMax::MSR, MicroArchitecture::CometLake),

            /* Tremont */
            0x86 => (CpuTJMax::MSR, MicroArchitecture::Tremont),

            /* Tiger Lake */
            0x8C | 0x8D => (CpuTJMax::MSR, MicroArchitecture::TigerLake),

            /* Alder Lake */
            0x97 | 0x9A | 0xBE => (CpuTJMax::MSR, MicroArchitecture::AlderLake),

            /* Raptor Lake */
            0xB7 | 0xBA | 0xBF => (CpuTJMax::MSR, MicroArchitecture::RaptorLake),

            /* Meteor Lake */
            0xAC | 0xAA => (CpuTJMax::MSR, MicroArchitecture::MeteorLake),

            /* Jasper Lake */
            0x9C => (CpuTJMax::MSR, MicroArchitecture::JasperLake),

            /* Rocket Lake */
            0xA7 => (CpuTJMax::MSR, MicroArchitecture::RocketLake),

            /* Arrow Lake */
            0xC5 | 0xC6 => (CpuTJMax::MSR, MicroArchitecture::ArrowLake),

            /* Lunar Lake */
            0xBD => (CpuTJMax::MSR, MicroArchitecture::LunarLake),

            /* Sapphire Rapids */
            0x8F => (CpuTJMax::MSR, MicroArchitecture::SapphireRapids),

            /* Elkhart Lake */
            0x96 => (CpuTJMax::MSR, MicroArchitecture::ElkhartLake),

            _ => (CpuTJMax::Static(100.0), MicroArchitecture::Unknown),
        },

        0x0F => match model_id {
            /* NetBurst
             * 0x00: Pentium 4 (180nm)
             * 0x01: Pentium 4 (130nm)
             * 0x02: Pentium 4 (130nm)
             * 0x03: Pentium 4, Celeron D (90nm)
             * 0x04: Pentium 4, Pentium D, Celeron D (90nm)
             * 0x06: Pentium 4, Pentium D, Celeron D (65nm)
             */
            0x00 | 0x01 | 0x02 | 0x03 | 0x04 | 0x06 => {
                (CpuTJMax::Static(100.0), MicroArchitecture::NetBurst)
            }

            _ => (CpuTJMax::Static(100.0), MicroArchitecture::Unknown),
        },

        _ => (CpuTJMax::Static(100.0), MicroArchitecture::Unknown),
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

impl Cpu {
    pub fn discover(system: &mut System) -> Result<Vec<Cpu>, String> {
        let affinities = get_all_group_affinities()?;
        let mut cpus: Vec<Cpu> = Vec::new();

        let results = run_on_all_affinities(affinities, |affinity| detect_cpu(affinity))?;

        // let _ = run_on_all_affinities(affinities, |_| {
        //     let mut creation = FILETIME::default();
        //     let mut exit = FILETIME::default();
        //     let mut kernel = FILETIME::default();
        //     let mut user = FILETIME::default();

        //     unsafe {
        //         let mut sum = 0u64;
        //         for i in 0..10_000_000_0 {
        //             sum = sum.wrapping_add(i);
        //         }
        //         let ok = GetThreadTimes(
        //             GetCurrentThread(),
        //             &mut creation,
        //             &mut exit,
        //             &mut kernel,
        //             &mut user,
        //         )
        //         .is_ok();
        //         println!("dummy sum = {}", sum);

        //         if ok {
        //             let kernel_ns =
        //                 ((kernel.dwHighDateTime as u64) << 32 | kernel.dwLowDateTime as u64) * 100;
        //             let user_ns =
        //                 ((user.dwHighDateTime as u64) << 32 | user.dwLowDateTime as u64) * 100;
        //             println!("Thread kernel_ns: {}, user_ns: {}", kernel_ns, user_ns);
        //         }
        //     }
        //     ()
        // });

        // return Err("()".into());

        let mut root_affinity = HashMap::new();

        let mut cores_per_package: HashMap<u32, HashSet<u32>> = HashMap::new();

        for (_affinity, info, _features, has_dts) in &results {
            let (package_id, core_id, _smt_id, _vendor, _model) = info.clone().expect("msg");

            cores_per_package
                .entry(package_id)
                .or_default()
                .insert(core_id);
        }
        let backend = Rc::new(RefCell::new(CpuLoadBackend::new(results.len())));

        for (affinity, info, features, has_dts) in results {
            let (package_id, core_id, smt_id, vendor, model) = info?;

            root_affinity
                .entry(package_id)
                .and_modify(|existing| {
                    if is_lower_affinity(&affinity, existing) {
                        *existing = affinity.clone();
                    }
                })
                .or_insert_with(|| affinity.clone());

            if let Some(cpu) = cpus.iter_mut().find(|c| c.package_id == package_id) {
                if let Some(core) = cpu.cores.iter_mut().find(|c| c.core_id == core_id) {
                    system.add_sensor(Sensor {
                        id: format!(
                            "/cpu/{}/core/{}/thread/{}/load",
                            package_id, core_id, smt_id
                        ),
                        impl_: Box::new(ThreadLoadSensor::new(&backend, affinity.clone())),
                        parameters: None,
                    });
                    core.threads.push(Thread::new(smt_id, affinity));
                } else {
                    let mut core = Core::new(core_id);

                    // Get the driver, insert if missing
                    let driver = match system.get_driver::<IntelMsr>() {
                        Some(driver) => driver,
                        None => system.insert_driver(IntelMsr::new()),
                    };

                    let tj_max_p = get_tj_max_from_msr(&driver, affinity);

                    // Add sensor for this package
                    system.add_sensor(Sensor {
                        id: format!("/cpu/{}/core/{}/temp", package_id, core_id),
                        impl_: Box::new(IntelCpuTempSensor {
                            driver: driver.clone(), // clone Rc
                            affinity: affinity.clone(),
                        }),
                        parameters: Some(vec![tj_max_p, 1.0]),
                    });

                    system.add_sensor(Sensor {
                        id: format!(
                            "/cpu/{}/core/{}/thread/{}/load",
                            package_id, core_id, smt_id
                        ),
                        impl_: Box::new(ThreadLoadSensor::new(&backend, affinity.clone())),
                        parameters: None,
                    });

                    core.threads.push(Thread::new(smt_id, affinity));
                    cpu.cores.push(core);
                }
            } else {
                let mut core = Core::new(core_id);

                // Get the driver, insert if missing
                let driver = match system.get_driver::<IntelMsr>() {
                    Some(driver) => driver,
                    None => system.insert_driver(IntelMsr::new()),
                };

                system.add_sensor(Sensor {
                    id: format!(
                        "/cpu/{}/core/{}/thread/{}/load",
                        package_id, core_id, smt_id
                    ),
                    impl_: Box::new(ThreadLoadSensor::new(&backend, affinity.clone())),
                    parameters: None,
                });

                core.threads.push(Thread::new(smt_id, affinity.clone()));
                let family_id = features.family_id();
                let model_id = features.model_id();
                let stepping_id = features.stepping_id();

                let (tj_max, micro_architecture) = get_tj_max(
                    family_id,
                    model_id,
                    stepping_id,
                    cores_per_package.get(&package_id).unwrap().len(),
                );

                let tj_max_p = get_tj_max_from_msr(&driver, affinity);

                // Add sensor for this package
                system.add_sensor(Sensor {
                    id: format!("/cpu/{}/core/{}/temp", package_id, core_id),
                    impl_: Box::new(IntelCpuTempSensor {
                        driver: driver.clone(), // clone Rc
                        affinity: affinity.clone(),
                    }),
                    parameters: Some(vec![tj_max_p, 1.0]),
                });

                println!("micro_architecture: {:?}", micro_architecture);

                cpus.push(Cpu {
                    family_id,
                    model_id,
                    stepping_id,
                    package_id,
                    vendor,
                    model,
                    affinity,
                    cores: vec![core],
                    tj_max,
                    micro_architecture,
                });
            }
        }

        for (package_id, affinity) in &root_affinity {
            // println!("pkg {} temp affinity: {:#?}", package_id, affinity);

            // Get the driver, insert if missing
            let driver = match system.get_driver::<IntelMsr>() {
                Some(driver) => driver,
                None => system.insert_driver(IntelMsr::new()),
            };

            // TODO: if dts and if known acrh
            // Add sensor for this package
            system.add_sensor(Sensor {
                id: format!("/cpu/{}/temp", package_id),
                impl_: Box::new(IntelCpuTempSensor {
                    driver: driver.clone(), // clone Rc
                    affinity: affinity.clone(),
                }),
                // TODO: get core => core_id = 0
                parameters: Some(vec![]),
            });
        }

        // println!("cpus: {:#?}", cpus);
        system.register_backend(&(backend as Rc<RefCell<dyn Backend>>));

        Ok(cpus)
    }
}

fn detect_cpu(
    affinity: GroupAffinity,
) -> (
    GroupAffinity,
    Result<(u32, u32, u32, Vendor, String), String>,
    FeatureInfo,
    bool,
) {
    let cpuid = CpuId::new();
    let vendor = get_vendor(&cpuid);
    let model = get_model(&cpuid);
    let features = cpuid.get_feature_info().unwrap();
    // Try to get thermal and power info (CPUID leaf 0x06)
    let has_dts = cpuid
        .get_thermal_power_info() // returns Option<ThermalPowerInfo>
        .map(|t| t.has_dts()) // map to bool
        .unwrap_or(false); // default false if CPUID leaf not supported

    let info = if let Some(topoiter) = cpuid.get_extended_topology_info() {
        get_topology_info(topoiter, vendor, &model)
    } else {
        get_legacy_info(&cpuid, vendor, &model)
    };

    (affinity, info, features, has_dts)
}

fn get_model<R: CpuIdReader>(cpuid: &CpuId<R>) -> String {
    cpuid
        .get_processor_brand_string()
        .map(|s| s.as_str().to_string())
        .unwrap_or_default()
}

#[derive(Debug)]
pub struct IntelCpuTempSensor {
    pub driver: Rc<RefCell<IntelMsr>>,
    pub affinity: GroupAffinity,
}

impl SensorImpl for IntelCpuTempSensor {
    fn read(&self, parameters: &Option<Vec<f32>>) -> Result<f32, String> {
        let (eax, _) = self
            .driver
            .borrow()
            .read_msr_affinity(IA32_PACKAGE_THERM_STATUS, &self.affinity)?;

        if (eax & 0x8000_0000) == 0 {
            return Err("Unknown value".into());
        }

        let params = parameters.as_deref();

        let tj_max = params.and_then(|v| v.get(0)).copied().unwrap_or(100.0);

        let slope = params.and_then(|v| v.get(1)).copied().unwrap_or(1.0);

        let delta_t = ((eax & 0x007F_0000) >> 16) as f32;

        Ok(tj_max - slope * delta_t)
    }

    fn kind(&self) -> SensorKind {
        SensorKind::Temperature
    }
    fn target(&self) -> SensorTarget {
        SensorTarget::Cpu
    }
}
