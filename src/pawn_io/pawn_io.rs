use std::ffi::c_void;
use std::path::Path;
use std::ptr::null_mut;
use std::{fs, slice};
use windows::core::PCSTR;
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileA, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;
use winreg::enums::*;
use winreg::RegKey;
use x86::msr::IA32_PACKAGE_THERM_STATUS;

const DEVICE_TYPE: u32 = 41394 << 16;
const FN_NAME_LENGTH: usize = 32;
const IOCTL_PIO_EXECUTE_FN: u32 = 0x841 << 2;
const IOCTL_PIO_LOAD_BINARY: u32 = 0x821 << 2;

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

#[derive(Debug)]
pub struct PawnIo {
    handle: Option<HANDLE>,
}

impl PawnIo {
    pub fn load_module_from_file(filename: &str) -> Self {
        let resource_bytes = match read_resource_bytes(filename) {
            Some(bytes) => bytes,
            None => return Self { handle: None },
        };

        Self::load_module_from_bytes(&resource_bytes)
    }

    fn load_module_from_bytes(resource_bytes: &[u8]) -> Self {
        unsafe {
            let handle = match CreateFileA(
                PCSTR(b"\\\\.\\PawnIO\0".as_ptr()),
                (GENERIC_READ | GENERIC_WRITE).0,   // desired access
                FILE_SHARE_READ | FILE_SHARE_WRITE, // share mode
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            ) {
                Ok(h) => h,
                Err(err) => {
                    println!("{:#?}", err);
                    return Self { handle: None };
                }
            };

            let mut bytes_written = 0u32;
            let result = DeviceIoControl(
                handle,
                DEVICE_TYPE | IOCTL_PIO_LOAD_BINARY,
                Some(resource_bytes.as_ptr() as *mut _),
                resource_bytes.len() as u32,
                None,
                0,
                Some(&mut bytes_written),
                None,
            );

            if result.is_ok() {
                Self {
                    handle: Some(handle),
                }
            } else {
                let _ = CloseHandle(handle);
                Self { handle: None }
            }
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.handle.is_some()
    }

    pub fn execute<TIn: AsBytes, TOut: AsMutBytes>(
        &self,
        name: &str,
        in_buffer: Option<TIn>,
        out_buffer: Option<TOut>,
    ) -> Result<(), String> {
        let handle = match self.handle {
            Some(h) => h,
            None => return Err("Driver not opened!".to_string()), // "not loaded"
        };

        // Prepare input
        let (in_ptr, in_len) = if let Some(buf) = in_buffer {
            (buf.ptr() as *mut c_void, buf.len_bytes())
        } else {
            (null_mut(), 0)
        };

        let mut total_input = vec![0u8; FN_NAME_LENGTH + in_len];
        let name_bytes = name.as_bytes();
        total_input[..name_bytes.len().min(FN_NAME_LENGTH - 1)]
            .copy_from_slice(&name_bytes[..name_bytes.len().min(FN_NAME_LENGTH - 1)]);

        if in_len > 0 {
            total_input[FN_NAME_LENGTH..]
                .copy_from_slice(unsafe { std::slice::from_raw_parts(in_ptr as *mut u8, in_len) });
        }

        // Prepare output
        let (out_ptr, out_len) = if let Some(mut buf) = out_buffer {
            (buf.ptr() as *mut c_void, buf.len_bytes() as u32)
        } else {
            (null_mut(), 0)
        };

        let mut bytes_returned: u32 = 0;

        unsafe {
            DeviceIoControl(
                handle,
                DEVICE_TYPE | IOCTL_PIO_EXECUTE_FN,
                Some(total_input.as_mut_ptr() as *mut _),
                total_input.len() as u32,
                Some(out_ptr),
                out_len,
                Some(&mut bytes_returned),
                None,
            )
            .map_err(|err| {
                format!(
                    "DeviceIoControl - Unable to write command {}. Last error code: {}",
                    name,
                    err.code().0 & 0xffff
                )
            })?;
        }

        Ok(())
    }

    pub fn execute_hr(
        &self,
        name: &str,
        in_buffer: &[i64],
        in_size: usize,
        out_buffer: &mut [i64],
        out_size: usize,
    ) -> Result<usize, u32> {
        // Check buffer sizes and return error instead of panicking
        if in_buffer.len() < in_size {
            return Err(1); // custom error code for "input too small"
        }
        if out_buffer.len() < out_size {
            return Err(2); // custom error code for "output too small"
        }

        let handle = match self.handle {
            Some(h) => h,
            None => return Err(3), // "not loaded"
        };

        self.execute_internal(name, &in_buffer[..in_size], out_size)
            .map(|res| {
                let copy_len = usize::min(res.len(), out_buffer.len());
                out_buffer[..copy_len].copy_from_slice(&res[..copy_len]);
                copy_len
            })
            .map_err(|err| err)
    }

    fn execute_internal(
        &self,
        name: &str,
        input: &[i64],
        out_length: usize,
    ) -> Result<Vec<i64>, u32> {
        let handle = match self.handle {
            Some(h) => h,
            None => return Err(0),
        };

        let mut output = vec![0u8; out_length * std::mem::size_of::<i64>()];
        let mut total_input = vec![0u8; FN_NAME_LENGTH + input.len() * std::mem::size_of::<i64>()];

        let name_bytes = name.as_bytes();
        total_input[..name_bytes.len().min(FN_NAME_LENGTH - 1)].copy_from_slice(name_bytes);

        total_input[FN_NAME_LENGTH..].copy_from_slice(unsafe {
            std::slice::from_raw_parts(
                input.as_ptr() as *const u8,
                input.len() * std::mem::size_of::<i64>(),
            )
        });

        let mut bytes_returned = 0u32;
        unsafe {
            let result = DeviceIoControl(
                handle,
                DEVICE_TYPE | IOCTL_PIO_EXECUTE_FN,
                Some(total_input.as_mut_ptr() as *mut _),
                total_input.len() as u32,
                Some(output.as_mut_ptr() as *mut _),
                output.len() as u32,
                Some(&mut bytes_returned),
                None,
            );

            match result {
                Ok(_) => {
                    let out_count = (bytes_returned as usize) / std::mem::size_of::<i64>();
                    let mut out_vec = vec![0i64; out_count];
                    std::ptr::copy_nonoverlapping(
                        output.as_ptr() as *const i64,
                        out_vec.as_mut_ptr(),
                        out_count,
                    );
                    Ok(out_vec)
                }
                Err(e) => {
                    let code_u32 = (e.code().0 as u32) & 0xFFFF;
                    Err(code_u32)
                }
            }
        }
    }

    pub fn close(&mut self) {
        if let Some(handle) = self.handle {
            unsafe {
                let _ = CloseHandle(handle);
            }
            self.handle = None;
        }
    }
}

fn read_resource_bytes(filename: &str) -> Option<Vec<u8>> {
    let path = Path::new("resources/pawn_io").join(filename);
    fs::read(&path).ok()
}

pub fn get_pawnio_version() -> Option<String> {
    let path = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO";

    // normal registry view
    if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path) {
        if let Ok(ver) = key.get_value::<String, _>("DisplayVersion") {
            return Some(ver);
        }
    }

    // 64-bit Wow64 view
    if let Ok(key) =
        RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(path, KEY_READ | KEY_WOW64_64KEY)
    {
        if let Ok(ver) = key.get_value::<String, _>("DisplayVersion") {
            return Some(ver);
        }
    }

    None
}
