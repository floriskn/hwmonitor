use raw_cpuid::CpuIdReaderNative;

use crate::system::{
    cpu::{
        backends::backend_context::BackendContext,
        sensors::thread_load::ThreadLoadSensor,
        topology::{CoreNode, CpuNode, ThreadNode},
        vendor::CpuVendor,
    },
    sensor::Sensor,
    system::System,
};

pub struct UnknownVendor;

impl CpuVendor for UnknownVendor {
    fn register_cpu_sensors(
        &self,
        _system: &mut System,
        _cpu: &CpuNode<CpuIdReaderNative>,
        _backends: &mut BackendContext,
    ) {
        // No sensors for unknown CPUs
    }

    fn register_core_sensors(
        &self,
        _system: &mut System,
        _cpu: &CpuNode<CpuIdReaderNative>,
        _core: &CoreNode,
        _backends: &mut BackendContext,
    ) {
    }

    fn register_thread_sensors(
        &self,
        system: &mut System,
        cpu: &CpuNode<CpuIdReaderNative>,
        _core: Option<&CoreNode>,
        thread: &ThreadNode,
        backends: &BackendContext,
    ) {
        if cpu.package_id.is_some() {
            return;
        }

        system.add_sensor(Sensor {
            id: format!("/cpu/?/thread/{}/load", cpu.affinity.to_flat_index()),
            impl_: Box::new(ThreadLoadSensor::new(
                &backends.thread_backend,
                thread.affinity,
            )),
            parameters: None,
        });
    }
}
