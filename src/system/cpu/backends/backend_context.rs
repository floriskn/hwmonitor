use std::{any::TypeId, cell::RefCell, collections::HashMap, rc::Rc};

use crate::system::{
    backend::Backend, cpu::backends::common::thread_backend::ThreadBackend, system::System,
};

pub struct BackendContext {
    // Shared globally by ALL threads
    pub thread_backend: Rc<RefCell<ThreadBackend>>,

    // Map of CPU-specific backends
    cpu_backends: HashMap<TypeId, HashMap<Option<u32>, Rc<RefCell<dyn Backend>>>>,
}

impl BackendContext {
    pub fn new(thread_count: usize) -> Self {
        Self {
            thread_backend: Rc::new(RefCell::new(ThreadBackend::new(thread_count))),
            cpu_backends: HashMap::new(),
        }
    }

    // Insert backend for a specific CPU package
    pub fn insert<T: Backend + 'static>(
        &mut self,
        package_id: Option<u32>,
        backend: Rc<RefCell<T>>,
    ) {
        let type_map = self
            .cpu_backends
            .entry(TypeId::of::<T>())
            .or_insert_with(|| HashMap::new());

        type_map.insert(package_id, backend as Rc<RefCell<dyn Backend>>);
    }

    /// Get (or lazily create) a backend for a specific CPU package
    pub fn get_or_insert<T, F>(&mut self, package_id: Option<u32>, creator: F) -> Rc<RefCell<T>>
    where
        T: Backend + 'static,
        F: FnOnce() -> T,
    {
        if let Some(existing) = self.get::<T>(package_id) {
            return existing;
        }

        let backend = Rc::new(RefCell::new(creator()));
        self.insert(package_id, backend.clone());
        backend
    }

    /// Get a CPU-specific backend
    pub fn get<T: Backend + 'static>(&self, package_id: Option<u32>) -> Option<Rc<RefCell<T>>> {
        self.cpu_backends.get(&TypeId::of::<T>()).and_then(|map| {
            map.get(&package_id).and_then(|b| {
                // Safe to clone Rc
                let backend_rc = Rc::clone(b);

                let raw: *const RefCell<dyn Backend> = Rc::as_ptr(&backend_rc);
                let typed_rc: Rc<RefCell<T>> = unsafe { Rc::from_raw(raw as *const RefCell<T>) };
                std::mem::forget(backend_rc); // prevent double free
                Some(typed_rc)
            })
        })
    }

    // Register all CPU-specific backends with the system
    pub fn register_all(&self, system: &mut System) {
        for map in self.cpu_backends.values() {
            for backend in map.values() {
                system.register_backend(&(backend.clone() as Rc<RefCell<dyn Backend>>));
            }
        }
        system.register_backend(&(self.thread_backend.clone() as Rc<RefCell<dyn Backend>>));
    }
}
