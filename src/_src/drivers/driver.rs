use std::any::Any;

pub trait Driver: Any + std::fmt::Debug {
    fn shutdown(&mut self);
}
