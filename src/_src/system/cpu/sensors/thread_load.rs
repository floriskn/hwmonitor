use std::{cell::RefCell, rc::Rc, time::Instant};

use crate::_src::system::{
    cpu::{
        affinity::group::GroupAffinity,
        backends::thread_backend::{ThreadBackend, ThreadBackendError},
    },
    sensor::{SensorError, SensorImpl, SensorKind, SensorTarget},
};

#[derive(Debug)]
pub struct ThreadLoadSensor {
    backend: Rc<RefCell<ThreadBackend>>,
    index: usize,
}

impl ThreadLoadSensor {
    pub fn new(backend: &Rc<RefCell<ThreadBackend>>, affinity: GroupAffinity) -> Self {
        Self {
            backend: backend.clone(),
            index: affinity.to_flat_index(),
        }
    }
}

impl SensorImpl for ThreadLoadSensor {
    fn read(&self, _parameters: &Option<Vec<f32>>) -> Result<Option<f32>, SensorError> {
        self.backend.borrow().get(self.index).map_err(|e| match e {
            // Map Win32 errors to HardwareError
            ThreadBackendError::Win32(win_err) => SensorError::HardwareError(win_err.to_string()),
            // Map Out of Bounds to NotFound
            ThreadBackendError::IndexOutOfBounds { index, max } => {
                SensorError::NotFound(format!("Thread index {} exceeds max {}", index, max))
            }
        })
    }

    fn kind(&self) -> SensorKind {
        SensorKind::Utilization
    }

    fn target(&self) -> SensorTarget {
        SensorTarget::Cpu
    }
}
