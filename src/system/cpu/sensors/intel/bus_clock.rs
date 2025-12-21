use std::{cell::RefCell, rc::Rc};

use crate::system::{
    cpu::backends::time_stamp_counter_backend::TimeStampCounterBackend,
    sensor::{SensorError, SensorImpl, SensorKind, SensorTarget},
};

#[derive(Debug)]
pub struct IntelBusClockSensor {
    pub backend: Rc<RefCell<TimeStampCounterBackend>>,
    pub time_stamp_counter_multiplier: f64,
}

impl SensorImpl for IntelBusClockSensor {
    fn read(&self, _: &Option<Vec<f32>>) -> Result<Option<f32>, SensorError> {
        Ok(Some(
            (self.backend.borrow().time_stamp_counter_frequency
                / self.time_stamp_counter_multiplier) as f32,
        ))
    }

    fn kind(&self) -> SensorKind {
        SensorKind::Clock
    }

    fn target(&self) -> SensorTarget {
        SensorTarget::Cpu
    }
}
