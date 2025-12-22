use raw_cpuid::CpuIdReaderNative;

use crate::system::{
    cpu::{
        backends::backend_context::BackendContext,
        topology::{CoreNode, CpuNode, ThreadNode},
        vendor::CpuVendor,
    },
    system::System,
};

pub struct AmdVendor;

impl CpuVendor for AmdVendor {
    fn register_cpu_sensors(
        &self,
        _system: &mut System,
        _cpu: &CpuNode<CpuIdReaderNative>,
        _backends: &mut BackendContext,
    ) {
        // Implement AMD package-level sensors (PPT, Tctl, etc)
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
