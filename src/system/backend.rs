use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Weak;

pub trait Backend {
    fn update(&mut self);
}

#[derive(Debug, Clone)]
pub struct BackendRef(pub Weak<RefCell<dyn Backend>>);

impl PartialEq for BackendRef {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

impl Eq for BackendRef {}

impl Hash for BackendRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Use the raw pointer address for the hash to uniquely identify the allocation
        self.0.as_ptr().hash(state);
    }
}
