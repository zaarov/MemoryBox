use std::mem;
use std::ptr;

use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::ProcessStatus::{GetModuleInformation, MODULEINFO};
use windows_sys::Win32::System::Threading::GetCurrentProcess;
use windows_sys::core::BOOL;

use crate::MemoryError;
use crate::local_ptr::LocalPtr;
use crate::scan::scan_bytes;

#[derive(Debug, Clone, Copy)]
pub struct ModuleContext {
    pub module_base: usize,
    pub module_size: usize,
}

impl ModuleContext {
    pub fn current() -> Result<Self, MemoryError> {
        unsafe {
            let h_mod: HMODULE = GetModuleHandleW(ptr::null());

            if h_mod.is_null() {
                return Err(MemoryError::ReadFailed);
            }

            let mut info: MODULEINFO = MODULEINFO {
                lpBaseOfDll: ptr::null_mut(),
                SizeOfImage: 0,
                EntryPoint: ptr::null_mut(),
            };

            let ok: i32 = GetModuleInformation(
                GetCurrentProcess(),
                h_mod,
                &mut info as *mut MODULEINFO,
                mem::size_of::<MODULEINFO>() as u32,
            );

            if ok == 0 {
                return Err(MemoryError::ReadFailed);
            }

            Ok(ModuleContext {
                module_base: info.lpBaseOfDll as usize,
                module_size: info.SizeOfImage as usize,
            })
        }
    }

    pub fn pattern_scan(&self, pattern: &[Option<u8>]) -> Result<LocalPtr, MemoryError> {
        unsafe {
            let base: usize = self.module_base;
            let size: usize = self.module_size;
            if size == 0 || base == 0 {
                return Err(MemoryError::OutOfBounds);
            }

            let h_proc: *mut std::ffi::c_void = GetCurrentProcess();
            let mut buffer: Vec<u8> = vec![0u8; size];
            let mut bytes_read: usize = 0;

            let ok: BOOL = ReadProcessMemory(
                h_proc,
                base as *const _,
                buffer.as_mut_ptr() as *mut _,
                size,
                &mut bytes_read as *mut usize,
            );

            if ok == 0 || bytes_read == 0 {
                return Err(MemoryError::ReadFailed);
            }

            buffer.truncate(bytes_read);

            if let Some(idx) = scan_bytes(&buffer, pattern) {
                Ok(LocalPtr {
                    address: base + idx,
                })
            } else {
                Err(MemoryError::ReadFailed)
            }
        }
    }
}
