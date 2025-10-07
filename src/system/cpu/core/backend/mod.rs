pub mod amd;
pub mod intel;
pub mod unknown;

pub trait CoreBackend {
    fn read_temp(&self) -> Result<f32, String>;
}
