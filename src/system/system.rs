use std::{
    any::TypeId,
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    drivers::driver::Driver,
    system::{
        backend::{Backend, BackendRef},
        sensor::Sensor,
    },
};

#[derive(Debug)]
pub struct System {
    // Todo: Weak ref?
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

    pub fn discover(&mut self) -> Result<(), String> {
        // TODO: store in system
        // let _ = Cpu::discover(self);

        super::cpu::register::register_all(self);

        Ok(())
    }

    /// Add a sensor to the system
    pub(crate) fn add_sensor(&mut self, sensor: Sensor) {
        self.sensors.push(sensor);
    }

    pub(crate) fn register_backend(&mut self, backend: &Rc<RefCell<dyn Backend>>) {
        self.backends.insert(BackendRef(Rc::downgrade(backend)));
    }

    pub fn update_backends(&mut self) {
        self.backends.retain(|b| {
            if let Some(backend) = b.0.upgrade() {
                backend.borrow_mut().update();
                true
            } else {
                false
            }
        });
    }

    pub(crate) fn insert_driver<D: Driver + 'static>(&mut self, driver: D) -> Rc<RefCell<D>> {
        println!("DRIVER CREATED");
        let type_id = std::any::TypeId::of::<D>();
        let rc = Rc::new(RefCell::new(driver));
        self.drivers.insert(type_id, rc.clone());
        rc
    }

    /// Check if a driver of this type exists
    pub(crate) fn has_driver<D: Driver + 'static>(&self) -> bool {
        self.drivers.contains_key(&TypeId::of::<D>())
    }

    /// Get a driver if it exists
    pub(crate) fn get_driver<D: Driver + 'static>(&self) -> Option<Rc<RefCell<D>>> {
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
