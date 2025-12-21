#[derive(Debug, Clone)]
pub enum CpuTJMax {
    MSR,
    Static(f32),
}
