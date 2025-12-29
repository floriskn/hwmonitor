use raw_cpuid::CpuIdReaderNative;

use crate::{
    drivers::pawn_io::amd_family_0f::AmdFamily0F,
    system::{
        cpu::{
            backends::{
                backend_context::BackendContext,
                common::time_stamp_counter_backend::TimeStampCounterBackend,
            },
            sensors::amd::bus_clock::AmdBusClockSensor,
            topology::{CoreNode, CpuNode, ThreadNode},
            vendor::CpuVendor,
        },
        sensor::Sensor,
        system::System,
    },
};

pub enum AmdVendor {
    F0F { offset: f32 }, // family 0x0F, holds offset
    F10,                 // family 0x10
    F17,                 // family 0x17
}

impl CpuVendor for AmdVendor {
    fn register_cpu_sensors(
        &self,
        system: &mut System,
        cpu: &CpuNode<CpuIdReaderNative>,
        backends: &mut BackendContext,
    ) {
        match self {
            AmdVendor::F0F { .. } => {
                if let Some(f_info) = cpu.cpuid.get_feature_info() {
                    if f_info.has_tsc() {
                        let driver = system
                            .get_driver::<AmdFamily0F>()
                            .unwrap_or_else(|| system.insert_driver(AmdFamily0F::new()));

                        const FIDVID_STATUS: u32 = 0xC0010042;
                        let msr_read = driver.borrow().read_msr(FIDVID_STATUS);

                        if msr_read.is_ok() {
                            let tsc_backend = backends.get_or_insert(cpu.package_id, || {
                                TimeStampCounterBackend::new(cpu.affinity.clone())
                            });

                            system.add_sensor(Sensor {
                                id: format!("/cpu/{}/clock", cpu.package_id.unwrap_or(0)),
                                impl_: Box::new(AmdBusClockSensor {
                                    backend: tsc_backend,
                                    driver,
                                }),
                                parameters: None,
                            });
                        }
                    }
                }
            }
            AmdVendor::F10 => todo!(),
            AmdVendor::F17 => todo!(),
        }
    }

    fn register_core_sensors(
        &self,
        _system: &mut System,
        _cpu: &CpuNode<CpuIdReaderNative>,
        _core: &CoreNode,
        _backends: &mut BackendContext,
    ) {
        // Implement AMD core-level sensors
    }

    fn register_thread_sensors(
        &self,
        _system: &mut System,
        _cpu: &CpuNode<CpuIdReaderNative>,
        _core: Option<&CoreNode>,
        _thread: &ThreadNode,
        _backends: &BackendContext,
    ) {
        // Optional thread sensors for AMD
    }
}
