pub mod amd;
pub mod intel;
pub mod unknown;

pub trait CpuBackend {
    fn read_package_temp(&self, package_id: u32) -> Result<f32, String>;
    fn read_core_temp(&self, core_id: u32) -> Option<f32>;
    fn read_thread_load(&self, thread_id: u32) -> Option<f32>;
    fn read_power(&self, package_id: u32) -> Option<f32>;
    fn read_voltage(&self, core_id: u32) -> Option<f32>;
}
