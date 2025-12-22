use std::{cell::RefCell, rc::Rc, time::Instant};

use raw_cpuid::CpuIdReaderNative;
use x86::msr::{
    IA32_PERF_STATUS, MSR_DRAM_ENERGY_STATUS, MSR_PKG_ENERGY_STATUS, MSR_PP0_ENERGY_STATUS,
    MSR_PP1_ENERGY_STATUS, MSR_RAPL_POWER_UNIT,
};

use crate::{
    drivers::pawn_io::intel_msr::IntelMsr,
    system::{
        cpu::{
            affinity::group::GroupAffinity,
            backends::{
                backend_context::BackendContext,
                common::{
                    thread_backend::ThreadBackend,
                    time_stamp_counter_backend::TimeStampCounterBackend,
                },
            },
            intel::{micro_architecture::MicroArchitecture, tj_max::CpuTJMax},
            sensors::{
                intel::{
                    bus_clock::IntelBusClockSensor, power::IntelCpuPowerSensor,
                    temparature::IntelCpuTempSensor, voltage::IntelCpuVoltageSensor,
                },
                thread_load::ThreadLoadSensor,
            },
            topology::{CoreNode, CpuNode, ThreadNode},
            vendor::CpuVendor,
        },
        sensor::Sensor,
        system::System,
    },
};

/// Intel-specific vendor implementation
pub struct IntelVendor {
    pub micro_arch: MicroArchitecture,
    pub tsc_multiplier: f64,
    pub tj_max: CpuTJMax,
}

impl CpuVendor for IntelVendor {
    fn register_cpu_sensors(
        &self,
        system: &mut System,
        cpu: &CpuNode<CpuIdReaderNative>,
        backends: &mut BackendContext,
    ) {
        if self.tsc_multiplier > 0.0 {
            let tsc_backend = backends.get_or_insert(cpu.package_id, || {
                TimeStampCounterBackend::new(cpu.affinity.clone())
            });
            system.add_sensor(Sensor {
                id: format!("/cpu/{}/clock", cpu.package_id.unwrap_or(0)),
                impl_: Box::new(IntelBusClockSensor {
                    backend: tsc_backend,
                    time_stamp_counter_multiplier: self.tsc_multiplier,
                }),
                parameters: None,
            });
        }

        if let Some(tp_info) = cpu.cpuid.get_thermal_power_info() {
            if self.micro_arch != MicroArchitecture::Unknown && tp_info.has_dts() {
                let driver = system
                    .get_driver::<IntelMsr>()
                    .unwrap_or_else(|| system.insert_driver(IntelMsr::new()));

                let tj_max = match self.tj_max {
                    CpuTJMax::MSR => get_tj_max_from_msr(&driver, &cpu.affinity),
                    CpuTJMax::Static(v) => v,
                };

                // Package temp sensor
                system.add_sensor(Sensor {
                    id: format!("/cpu/{}/temp", cpu.package_id.unwrap_or(0)),
                    impl_: Box::new(IntelCpuTempSensor {
                        driver,
                        affinity: cpu.affinity.clone(),
                    }),
                    parameters: Some(vec![tj_max, 1.0]),
                });
            }
        }

        let driver: Rc<RefCell<IntelMsr>> = system
            .get_driver::<IntelMsr>()
            .unwrap_or_else(|| system.insert_driver(IntelMsr::new()));

        // Attempt to read IA32_PERF_STATUS
        let msr_result = driver.borrow().read_msr(IA32_PERF_STATUS);

        // Drop the borrow immediately
        if let Ok((_, edx)) = msr_result {
            // Check if digital voltage is non-zero
            if edx & 0xFFFF > 0 {
                system.add_sensor(Sensor {
                    id: format!("/cpu/{}/voltage", cpu.package_id.unwrap_or(0)),
                    impl_: Box::new(IntelCpuVoltageSensor {
                        driver: driver, // clone Rc so we can still use driver later
                        affinity: None,
                    }),
                    parameters: None,
                });
            }
        }

        match self.micro_arch {
            MicroArchitecture::Airmont
            | MicroArchitecture::AlderLake
            | MicroArchitecture::ArrowLake
            | MicroArchitecture::Broadwell
            | MicroArchitecture::CannonLake
            | MicroArchitecture::CometLake
            | MicroArchitecture::Goldmont
            | MicroArchitecture::GoldmontPlus
            | MicroArchitecture::Haswell
            | MicroArchitecture::IceLake
            | MicroArchitecture::IvyBridge
            | MicroArchitecture::JasperLake
            | MicroArchitecture::KabyLake
            | MicroArchitecture::LunarLake
            | MicroArchitecture::MeteorLake
            | MicroArchitecture::RaptorLake
            | MicroArchitecture::RocketLake
            | MicroArchitecture::SandyBridge
            | MicroArchitecture::Silvermont
            | MicroArchitecture::Skylake
            | MicroArchitecture::TigerLake
            | MicroArchitecture::SapphireRapids
            | MicroArchitecture::ElkhartLake
            | MicroArchitecture::Tremont => {
                let driver: Rc<RefCell<IntelMsr>> = system
                    .get_driver::<IntelMsr>()
                    .unwrap_or_else(|| system.insert_driver(IntelMsr::new()));
                let msr_result = driver.borrow().read_msr(MSR_RAPL_POWER_UNIT);

                // Drop the borrow immediately
                if let Ok((eax, _)) = msr_result {
                    // Check if digital voltage is non-zero

                    let energy_units_multiplier = match self.micro_arch {
                        MicroArchitecture::Airmont | MicroArchitecture::Silvermont => {
                            1.0e-6f32 * (1 << (((eax >> 8) & 0x1F) as i32)) as f32
                        }
                        _ => 1.0 / (1 << (((eax >> 8) & 0x1F) as i32)) as f32,
                    };

                    let package_id = cpu.package_id.unwrap_or(0);

                    if energy_units_multiplier != 0.0 {
                        try_add_power_sensor(
                            MSR_PKG_ENERGY_STATUS,
                            system,
                            &driver,
                            package_id,
                            energy_units_multiplier,
                            "package_energy",
                        );
                        try_add_power_sensor(
                            MSR_PP0_ENERGY_STATUS,
                            system,
                            &driver,
                            package_id,
                            energy_units_multiplier,
                            "cores_energy",
                        );
                        // try_add_power_sensor(
                        //     MSR_PP1_ENERGY_STATUS,
                        //     system,
                        //     &driver,
                        //     package_id,
                        //     energy_units_multiplier,
                        //     "graphics_energy",
                        // );
                        try_add_power_sensor(
                            MSR_DRAM_ENERGY_STATUS,
                            system,
                            &driver,
                            package_id,
                            energy_units_multiplier,
                            "memory_energy",
                        );
                        const MSR_PLATFORM_ENERGY_STATUS: u32 = 0x64D;
                        try_add_power_sensor(
                            MSR_PLATFORM_ENERGY_STATUS, // 0x64D
                            system,
                            &driver,
                            package_id,
                            energy_units_multiplier,
                            "platform_energy",
                        );
                    }
                }
            }
            _ => {}
        }
    }

    fn register_core_sensors(
        &self,
        system: &mut System,
        cpu: &CpuNode<CpuIdReaderNative>,
        core: &CoreNode,
        _backends: &mut BackendContext,
    ) {
        if let Some(tp_info) = cpu.cpuid.get_thermal_power_info() {
            if self.micro_arch != MicroArchitecture::Unknown && tp_info.has_dts() {
                let driver = system
                    .get_driver::<IntelMsr>()
                    .unwrap_or_else(|| system.insert_driver(IntelMsr::new()));

                let tj_max = match self.tj_max {
                    CpuTJMax::MSR => get_tj_max_from_msr(&driver, &cpu.affinity),
                    CpuTJMax::Static(v) => v,
                };

                // Package temp sensor
                system.add_sensor(Sensor {
                    id: format!(
                        "/cpu/{}/core/{}/temp",
                        cpu.package_id.unwrap_or(0),
                        core.core_id
                    ),
                    impl_: Box::new(IntelCpuTempSensor {
                        driver,
                        affinity: cpu.affinity.clone(),
                    }),
                    parameters: Some(vec![tj_max, 1.0]),
                });
            }
        }

        let driver: Rc<RefCell<IntelMsr>> = system
            .get_driver::<IntelMsr>()
            .unwrap_or_else(|| system.insert_driver(IntelMsr::new()));

        // Attempt to read IA32_PERF_STATUS
        let msr_result = driver.borrow().read_msr(IA32_PERF_STATUS);

        // Drop the borrow immediately
        if let Ok((_, edx)) = msr_result {
            // Check if digital voltage is non-zero
            if edx & 0xFFFF > 0 {
                system.add_sensor(Sensor {
                    id: format!(
                        "/cpu/{}/core/{}/voltage",
                        cpu.package_id.unwrap_or(0),
                        core.core_id
                    ),
                    impl_: Box::new(IntelCpuVoltageSensor {
                        driver: driver, // clone Rc so we can still use driver later
                        affinity: Some(cpu.affinity.clone()),
                    }),
                    parameters: None,
                });
            }
        }
    }

    fn register_thread_sensors(
        &self,
        system: &mut System,
        cpu: &CpuNode<CpuIdReaderNative>,
        core: Option<&CoreNode>,
        thread: &ThreadNode,
        backends: &BackendContext,
    ) {
        let Some(package_id) = cpu.package_id else {
            return;
        };
        let Some(core) = core else {
            return;
        };

        // Add Thread Load Sensor
        system.add_sensor(Sensor {
            id: format!(
                "/cpu/{}/core/{}/thread/{}/load",
                package_id, core.core_id, thread.smt_id
            ),
            impl_: Box::new(ThreadLoadSensor::new(
                &backends.thread_backend,
                thread.affinity,
            )),
            parameters: None,
        });
    }
}

const IA32_TEMPERATURE_TARGET: u32 = 0x01A2;

fn get_tj_max_from_msr(driver: &Rc<RefCell<IntelMsr>>, group_affinity: &GroupAffinity) -> f32 {
    let res = driver
        .borrow()
        .read_msr_affinity(IA32_TEMPERATURE_TARGET, group_affinity);

    match res {
        Ok((eax, _)) => ((eax >> 16) & 0xFF) as f32,
        Err(_) => 100.0,
    }
}

fn try_add_power_sensor(
    index: u32,
    system: &mut System,
    driver: &Rc<RefCell<IntelMsr>>,
    package_id: u32,
    energy_units_multiplier: f32,
    id_suffix: &str,
) {
    // Attempt to read the MSR register
    let eax = match driver.borrow().read_msr(index) {
        Ok((eax, _)) if eax != 0 => eax,
        Ok(_) => return, // MSR returned 0, no sensor to add
        Err(_) => return,
    };

    // Add the sensor to the system
    system.add_sensor(Sensor {
        id: format!("/cpu/{}/{}", package_id, id_suffix),
        parameters: None,
        impl_: Box::new(IntelCpuPowerSensor {
            driver: Rc::clone(driver),
            energy_units_multiplier,
            index,
            last_energy_consumed: RefCell::new(eax),
            last_energy_time: RefCell::new(Instant::now()),
            value: RefCell::new(None),
        }),
    });
}
