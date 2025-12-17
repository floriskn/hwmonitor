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
}

pub trait SensorImpl: std::fmt::Debug {
    fn read(&self, parameters: &Option<Vec<f32>>) -> Result<f32, String>;
    fn kind(&self) -> SensorKind;
    fn target(&self) -> SensorTarget;
}

#[derive(Debug)]
pub struct Sensor {
    pub id: String,
    // Should be private
    pub parameters: Option<Vec<f32>>,
    pub impl_: Box<dyn SensorImpl>,
}

impl Sensor {
    pub fn read(&self) -> Result<f32, String> {
        self.impl_.read(&self.parameters)
    }

    pub fn kind(&self) -> SensorKind {
        self.impl_.kind()
    }

    pub fn target(&self) -> SensorTarget {
        self.impl_.target()
    }
}

// #[derive(Debug)]
// pub struct IntelCpuTempSensor {
//     driver: CpuDriver,
//     affinity: usize,
//     pub tj_max: f32,
//     pub slope: f32,
// }

// impl SensorImpl for IntelCpuTempSensor {
//     fn read(&self) -> Result<f32, String> {
//         let (eax, _) = self
//             .driver
//             .rdmsr_tx(IA32_PACKAGE_THERM_STATUS, &self.affinity)?;
//         if (eax & 0x8000_0000) != 0 {
//             let delta_t = ((eax & 0x007F_0000) >> 16) as f32;
//             Ok(self.tj_max - self.slope * delta_t)
//         } else {
//             Err("Unknown value".into())
//         }
//     }

//     fn kind(&self) -> SensorKind {
//         SensorKind::Temperature
//     }
//     fn target(&self) -> SensorTarget {
//         SensorTarget::Cpu
//     }
// }
