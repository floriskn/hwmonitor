#[derive(Debug, Clone)]
pub enum SensorError {
    /// The sensor exists but is currently failing to provide data (e.g., driver timeout).
    ReadFailure(String),
    /// The parameters provided to the sensor are invalid for this specific implementation.
    InvalidParameters(String),
    /// A hardware-level error (e.g., Win32 error, I2C failure, or disconnected device).
    HardwareError(String),
    /// The sensor implementation encountered an unexpected internal state.
    Internal(String),
    /// The specific index or sub-component requested does not exist.
    NotFound(String),
}

impl std::fmt::Display for SensorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadFailure(m) => write!(f, "Sensor read failure: {m}"),
            Self::InvalidParameters(m) => write!(f, "Invalid sensor parameters: {m}"),
            Self::HardwareError(m) => write!(f, "Hardware communication error: {m}"),
            Self::Internal(m) => write!(f, "Internal sensor error: {m}"),
            Self::NotFound(m) => write!(f, "Sensor target not found: {m}"),
        }
    }
}

impl std::error::Error for SensorError {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SensorTarget {
    Cpu,
    Gpu,
    Fan,
    Motherboard,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SensorKind {
    Temperature,
    Voltage,
    FanSpeed,
    Power,
    Utilization,
    Clock,
}

pub trait SensorImpl: std::fmt::Debug {
    fn read(&self, parameters: &Option<Vec<f32>>) -> Result<Option<f32>, SensorError>;
    fn kind(&self) -> SensorKind;
    fn target(&self) -> SensorTarget;
}

#[derive(Debug)]
pub struct Sensor {
    pub id: String,
    // pub value: Option<f32>,
    // Should be private
    pub parameters: Option<Vec<f32>>,
    pub impl_: Box<dyn SensorImpl>,
}

impl Sensor {
    pub fn read(&self) -> Result<Option<f32>, SensorError> {
        self.impl_.read(&self.parameters)
    }

    pub fn kind(&self) -> SensorKind {
        self.impl_.kind()
    }

    pub fn target(&self) -> SensorTarget {
        self.impl_.target()
    }
}
