pub(crate) mod amd;
pub(crate) mod intel;
pub(crate) mod unknown;

use raw_cpuid::CpuIdReaderNative;

use crate::system::{
    cpu::{
        backends::backend_context::BackendContext,
        intel::utils::{get_cpu_tjmax_info, get_time_stamp_counter_multiplier},
        topology::{CoreNode, CpuNode, ThreadNode},
        vendor::{amd::AmdVendor, intel::IntelVendor, unknown::UnknownVendor},
    },
    system::System,
};

#[derive(Debug, Clone)]
pub enum Vendor {
    Intel,
    Amd,
    Unknown(Option<String>),
}

pub trait CpuVendor {
    fn register_cpu_sensors(
        &self,
        system: &mut System,
        cpu: &CpuNode<CpuIdReaderNative>,
        backends: &mut BackendContext,
    );

    fn register_core_sensors(
        &self,
        system: &mut System,
        cpu: &CpuNode<CpuIdReaderNative>,
        core: &CoreNode,
        backends: &mut BackendContext,
    );

    fn register_thread_sensors(
        &self,
        system: &mut System,
        cpu: &CpuNode<CpuIdReaderNative>,
        core: Option<&CoreNode>,
        thread: &ThreadNode,
        backends: &BackendContext,
    );
}

pub fn vendor_for(
    system: &mut System,
    cpu_node: &CpuNode<CpuIdReaderNative>,
) -> Box<dyn CpuVendor> {
    match &cpu_node.vendor {
        Vendor::Intel { .. } => {
            let feature_info = cpu_node.cpuid.get_feature_info();
            let family_id = feature_info.as_ref().map_or(0, |f| f.family_id());
            let model_id = feature_info.as_ref().map_or(0, |f| f.model_id());
            let stepping_id = feature_info.as_ref().map_or(0, |f| f.stepping_id());

            // Compute values using existing helper functions
            let (tj_max, micro_arch) =
                get_cpu_tjmax_info(family_id, model_id, stepping_id, cpu_node.cores.len());

            let tsc_multiplier = get_time_stamp_counter_multiplier(system, &micro_arch);

            Box::new(IntelVendor {
                micro_arch,
                tsc_multiplier,
                tj_max,
            })
        }
        Vendor::Amd => Box::new(AmdVendor {}),
        Vendor::Unknown(_) => Box::new(UnknownVendor {}),
    }
}
