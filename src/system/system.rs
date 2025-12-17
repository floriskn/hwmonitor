use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    rc::{Rc, Weak},
    sync::Arc,
};

use crate::system::{
    cpu::cpu::Cpu,
    sensor::sensor::{Sensor, SensorKind},
};

pub trait Driver: Any + std::fmt::Debug {
    fn shutdown(&mut self);
}

pub trait Backend {
    fn update(&self);
}

#[derive(Debug, Clone)]
struct BackendRef(Weak<RefCell<dyn Backend>>);

impl PartialEq for BackendRef {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

impl Eq for BackendRef {}

impl Hash for BackendRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state);
    }
}

#[derive(Debug)]
pub struct System {
    drivers: HashMap<TypeId, Rc<RefCell<dyn Driver>>>,
    pub sensors: Vec<Sensor>,
    backends: HashSet<BackendRef>,
}

impl System {
    /// Create an empty system
    pub fn new() -> Self {
        Self {
            drivers: HashMap::new(),
            sensors: Vec::new(),
            backends: HashSet::new(),
        }
    }

    // 1. Return number of registered backends
    pub fn count_backends(&self) -> usize {
        self.backends.len()
    }

    // 2. Remove all Utilization sensors except one
    pub fn keep_one_utilization_sensor(&mut self) {
        let mut keep = false;
        self.sensors.retain(|s| {
            if s.kind() == SensorKind::Utilization {
                if keep {
                    return false; // remove
                } else {
                    keep = true;
                    return true; // keep the first one
                }
            }
            true // keep all others
        });
    }

    // 3. Remove all Utilization sensors
    pub fn remove_all_utilization_sensors(&mut self) {
        self.sensors.retain(|s| s.kind() != SensorKind::Utilization);
    }

    pub fn discover(&mut self) -> Result<(), String> {
        Cpu::discover(self)?;

        Ok(())
    }

    /// Add a sensor to the system
    pub fn add_sensor(&mut self, sensor: Sensor) {
        self.sensors.push(sensor);
    }

    pub(crate) fn register_backend(&mut self, backend: &Rc<RefCell<dyn Backend>>) {
        self.backends.insert(BackendRef(Rc::downgrade(backend)));
    }

    pub fn update_backends(&mut self) {
        self.backends.retain(|b| {
            if let Some(backend) = b.0.upgrade() {
                backend.borrow().update();
                true
            } else {
                false
            }
        });
    }

    pub fn insert_driver<D: Driver + 'static>(&mut self, driver: D) -> Rc<RefCell<D>> {
        println!("DRIVER CREATED");
        let type_id = std::any::TypeId::of::<D>();
        let rc = Rc::new(RefCell::new(driver));
        self.drivers.insert(type_id, rc.clone());
        rc
    }

    /// Check if a driver of this type exists
    pub fn has_driver<D: Driver + 'static>(&self) -> bool {
        self.drivers.contains_key(&TypeId::of::<D>())
    }

    /// Get a driver if it exists
    pub fn get_driver<D: Driver + 'static>(&self) -> Option<Rc<RefCell<D>>> {
        self.drivers.get(&TypeId::of::<D>()).map(|driver_rc| {
            // Safe to clone Rc
            let driver_rc = Rc::clone(driver_rc);

            // Convert Rc<RefCell<dyn Driver>> -> Rc<RefCell<D>>
            // Use dynamic borrow downcast
            let raw: *const RefCell<dyn Driver> = Rc::as_ptr(&driver_rc);
            let typed_rc: Rc<RefCell<D>> = unsafe { Rc::from_raw(raw as *const RefCell<D>) };
            std::mem::forget(driver_rc); // prevent double free
            typed_rc
        })
    }

    /// Close all drivers
    pub fn close(&self) {
        for driver in self.drivers.values() {
            driver.borrow_mut().shutdown();
        }
    }
}
