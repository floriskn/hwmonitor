use include_dir::{include_dir, Dir};
use std::ffi::c_void;
use std::ptr::null_mut;
use windows::core::{HRESULT, PCSTR};
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileA, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;
use winreg::enums::*;
use winreg::RegKey;

use super::utils::{AsBytes, AsMutBytes};

// Renamed to reflect system kernel binaries
static KERNEL_BINARIES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/resources/pawn_io");

const DEVICE_TYPE: u32 = 41394 << 16;
const FN_NAME_LENGTH: usize = 32;
const IOCTL_PIO_EXECUTE_FN: u32 = 0x841 << 2;
const IOCTL_PIO_LOAD_BINARY: u32 = 0x821 << 2;

#[derive(Debug)]
pub enum PawnIoError {
    Win32Error(windows::core::Error),
    ResourceNotFound(String),
    DriverNotLoaded,
    IoControlFailed { name: String, code: HRESULT },
}

#[derive(Debug)]
pub struct PawnIo {
    // Handle is now Option to support manual close()
    handle: Option<HANDLE>,
}

impl PawnIo {
    /// Attempts to load a module; if it fails (missing file or driver error),
    /// it returns an empty PawnIo instance instead of an error.
    pub fn load_module_from_file_or_empty(filename: &str) -> Self {
        Self::load_module_from_file(filename).unwrap_or_else(|_| Self { handle: None })
    }

    pub fn load_module_from_file(filename: &str) -> Result<Self, PawnIoError> {
        let file = KERNEL_BINARIES
            .get_file(filename)
            .ok_or_else(|| PawnIoError::ResourceNotFound(filename.to_string()))?;

        Self::load_module_from_bytes(file.contents())
    }

    fn load_module_from_bytes(resource_bytes: &[u8]) -> Result<Self, PawnIoError> {
        unsafe {
            let handle = CreateFileA(
                PCSTR(b"\\\\.\\PawnIO\0".as_ptr()),
                (GENERIC_READ | GENERIC_WRITE).0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
            .map_err(PawnIoError::Win32Error)?;

            if handle.is_invalid() {
                return Err(PawnIoError::DriverNotLoaded);
            }

            let mut bytes_written = 0u32;
            DeviceIoControl(
                handle,
                DEVICE_TYPE | IOCTL_PIO_LOAD_BINARY,
                Some(resource_bytes.as_ptr() as *mut _),
                resource_bytes.len() as u32,
                None,
                0,
                Some(&mut bytes_written),
                None,
            )
            .map_err(PawnIoError::Win32Error)?;

            Ok(Self {
                handle: Some(handle),
            })
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
    ) -> Result<(), PawnIoError> {
        // Guard against calling on a closed or uninitialized handle
        let handle = self.handle.ok_or(PawnIoError::DriverNotLoaded)?;

        let (in_ptr, in_len) = in_buffer
            .map(|buf| (buf.ptr() as *mut c_void, buf.len_bytes()))
            .unwrap_or((null_mut(), 0));

        let mut total_input = vec![0u8; FN_NAME_LENGTH + in_len];
        let name_bytes = name.as_bytes();
        let name_limit = name_bytes.len().min(FN_NAME_LENGTH - 1);
        total_input[..name_limit].copy_from_slice(&name_bytes[..name_limit]);

        if in_len > 0 {
            unsafe {
                let src = std::slice::from_raw_parts(in_ptr as *const u8, in_len);
                total_input[FN_NAME_LENGTH..].copy_from_slice(src);
            }
        }

        let (out_ptr, out_len) = out_buffer
            .map(|mut buf| (buf.ptr() as *mut c_void, buf.len_bytes() as u32))
            .unwrap_or((null_mut(), 0));

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
            .map_err(|err| PawnIoError::IoControlFailed {
                name: name.to_string(),
                code: err.code(),
            })?;
        }

        Ok(())
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

// Ensure handle is closed when struct is dropped
impl Drop for PawnIo {
    fn drop(&mut self) {
        self.close();
    }
}

pub fn get_pawnio_version() -> Option<String> {
    let path = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO";

    if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path) {
        if let Ok(ver) = key.get_value::<String, _>("DisplayVersion") {
            return Some(ver);
        }
    }

    if let Ok(key) =
        RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(path, KEY_READ | KEY_WOW64_64KEY)
    {
        if let Ok(ver) = key.get_value::<String, _>("DisplayVersion") {
            return Some(ver);
        }
    }

    None
}
