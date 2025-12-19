pub trait AsBytes {
    fn ptr(&self) -> *const u8;
    fn len_bytes(&self) -> usize;
}

// Single-value references
impl<T> AsBytes for &T {
    fn ptr(&self) -> *const u8 {
        *self as *const T as *const u8
    }
    fn len_bytes(&self) -> usize {
        std::mem::size_of::<u64>()
    }
}

// Slices
impl<T> AsBytes for &[T] {
    fn ptr(&self) -> *const u8 {
        self.as_ptr() as *const u8
    }
    fn len_bytes(&self) -> usize {
        self.len() * std::mem::size_of::<u64>()
    }
}

pub trait AsMutBytes {
    fn ptr(&mut self) -> *mut u8;
    fn len_bytes(&self) -> usize;
}

// Single-value mutable references
impl<T> AsMutBytes for &mut T {
    fn ptr(&mut self) -> *mut u8 {
        *self as *mut T as *mut u8
    }
    fn len_bytes(&self) -> usize {
        std::mem::size_of::<u64>()
    }
}

// Mutable slices
impl<T> AsMutBytes for &mut [T] {
    fn ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr() as *mut u8
    }
    fn len_bytes(&self) -> usize {
        self.len() * std::mem::size_of::<u64>()
    }
}
