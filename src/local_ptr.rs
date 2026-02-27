use std::ffi::c_void;
use std::mem::size_of;

use crate::error::MemoryError;

use windows_sys::Win32::System::Diagnostics::Debug::{
    FlushInstructionCache, ReadProcessMemory, WriteProcessMemory,
};
use windows_sys::Win32::System::Memory::{PAGE_PROTECTION_FLAGS, PAGE_READWRITE, VirtualProtect};
use windows_sys::Win32::System::Threading::GetCurrentProcess;
use windows_sys::core::BOOL;

pub type Address = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalPtr {
    pub address: Address,
}

impl LocalPtr {
    pub fn from_addr(address: Address) -> Self {
        LocalPtr { address }
    }

    pub fn offset(&self, off: isize) -> Result<LocalPtr, MemoryError> {
        if off >= 0 {
            let add: usize = off as usize;
            self.address
                .checked_add(add)
                .map(|a: usize| LocalPtr { address: a })
                .ok_or(MemoryError::OutOfBounds)
        } else {
            let sub: usize = (-off) as usize;
            if self.address < sub {
                Err(MemoryError::OutOfBounds)
            } else {
                Ok(LocalPtr {
                    address: self.address - sub,
                })
            }
        }
    }

    pub fn read_bytes(&self, len: usize) -> Result<Vec<u8>, MemoryError> {
        if len == 0 {
            return Ok(Vec::new());
        }

        unsafe {
            let h_proc: *mut c_void = GetCurrentProcess();
            let mut buf: Vec<u8> = vec![0u8; len];
            let mut bytes_read: usize = 0;

            let ok: BOOL = ReadProcessMemory(
                h_proc,
                self.address as *const c_void,
                buf.as_mut_ptr() as *mut c_void,
                len,
                &mut bytes_read as *mut usize,
            );

            if ok == 0 || bytes_read == 0 {
                return Err(MemoryError::ReadFailed);
            }

            if bytes_read != len {
                return Err(MemoryError::ReadFailed);
            }

            Ok(buf)
        }
    }

    pub fn deref(&self) -> Result<LocalPtr, MemoryError> {
        if cfg!(target_pointer_width = "64") {
            let b: Vec<u8> = self.read_bytes(size_of::<usize>())?;
            let ptr: usize =
                usize::from_le_bytes(b.try_into().map_err(|_| MemoryError::ReadFailed)?);
            Ok(LocalPtr { address: ptr })
        } else {
            let b: Vec<u8> = self.read_bytes(size_of::<u32>())?;
            let ptr32: usize =
                u32::from_le_bytes(b.try_into().map_err(|_| MemoryError::ReadFailed)?) as usize;
            Ok(LocalPtr { address: ptr32 })
        }
    }

    pub fn rip_relative(
        &self,
        offset_offset: isize,
        instruction_len: isize,
    ) -> Result<LocalPtr, MemoryError> {
        let disp_ptr: LocalPtr = self.offset(offset_offset)?;
        let bytes: Vec<u8> = disp_ptr.read_bytes(4)?;
        let disp: isize =
            i32::from_le_bytes(bytes.try_into().map_err(|_| MemoryError::ReadFailed)?) as isize;

        self.offset(instruction_len + disp)
    }

    pub fn write_bytes(&self, data: &[u8]) -> Result<(), MemoryError> {
        if data.is_empty() {
            return Ok(());
        }

        unsafe {
            let dst: *mut u8 = self.address as *mut u8;
            if dst.is_null() {
                return Err(MemoryError::NullPointer);
            }

            std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
            Ok(())
        }
    }

    pub fn write_bytes_protected(&self, data: &[u8]) -> Result<(), MemoryError> {
        if data.is_empty() {
            return Ok(());
        }

        unsafe {
            const PAGE_SIZE: usize = 0x1000;
            let start_page: usize = self.address & !(PAGE_SIZE - 1);

            let mut old_protect: PAGE_PROTECTION_FLAGS = 0;
            let ok_prot: i32 = VirtualProtect(
                start_page as *mut c_void,
                PAGE_SIZE,
                PAGE_READWRITE,
                &mut old_protect as *mut _,
            );
            if ok_prot == 0 {
                return Err(MemoryError::VirtualProtectFailed);
            }

            let h_proc: *mut c_void = GetCurrentProcess();
            let mut bytes_written: usize = 0;
            let ok_write: BOOL = WriteProcessMemory(
                h_proc,
                self.address as *mut c_void,
                data.as_ptr() as *const c_void as *mut c_void,
                data.len(),
                &mut bytes_written as *mut usize,
            );

            let _ = VirtualProtect(
                start_page as *mut c_void,
                PAGE_SIZE,
                old_protect,
                &mut old_protect as *mut _,
            );

            if ok_write == 0 || bytes_written != data.len() {
                return Err(MemoryError::WriteFailed);
            }

            let _ = FlushInstructionCache(h_proc, self.address as *const c_void, data.len());

            Ok(())
        }
    }

    pub fn chain(self) -> LocalPtrChain {
        LocalPtrChain { current: self }
    }
}

pub struct LocalPtrChain {
    current: LocalPtr,
}

impl LocalPtrChain {
    pub fn offset(mut self, off: isize) -> Result<Self, MemoryError> {
        self.current = self.current.offset(off)?;
        Ok(self)
    }

    pub fn deref(mut self) -> Result<Self, MemoryError> {
        self.current = self.current.deref()?;
        Ok(self)
    }

    pub fn finish(self) -> LocalPtr {
        self.current
    }
}
