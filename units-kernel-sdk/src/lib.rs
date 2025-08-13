#![cfg_attr(not(feature = "std"), no_std)]

//! UNITS Kernel SDK - Framework for building kernel modules in Rust
//! 
//! This SDK provides the necessary types and utilities for building
//! kernel modules that run in the UNITS RISC-V VM environment.
//!
//! # Memory Management
//! 
//! The SDK provides a safe allocator abstraction for kernel modules.
//! Use the `use_default_allocator!()` macro in your kernel module's main.rs
//! to avoid writing unsafe allocation code:
//! 
//! ```rust
//! #![no_std]
//! #![no_main]
//! 
//! use units_kernel_sdk::use_default_allocator;
//! 
//! use_default_allocator!();
//! ```

extern crate alloc;

pub mod allocator;

use alloc::vec::Vec;
use alloc::string::String;
use borsh::{BorshDeserialize, BorshSerialize};

#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as HashMap;
#[cfg(feature = "std")]
use std::collections::HashMap;

/// Size of object IDs in bytes
pub const OBJECT_ID_SIZE: usize = 32;

/// Units object ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BorshSerialize, BorshDeserialize)]
pub struct UnitsObjectId([u8; OBJECT_ID_SIZE]);

impl UnitsObjectId {
    pub const fn new(bytes: [u8; OBJECT_ID_SIZE]) -> Self {
        Self(bytes)
    }
    
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Object type enum
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum ObjectType {
    Data,
    Executable(VMType),
}

/// VM type enum
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum VMType {
    RiscV,
}

/// Units object structure
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct UnitsObject {
    pub id: UnitsObjectId,
    pub controller_id: UnitsObjectId,
    pub object_type: ObjectType,
    pub data: Vec<u8>,
}

/// Instruction structure
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct Instruction {
    pub controller_id: UnitsObjectId,
    pub target_function: String,
    pub target_objects: Vec<UnitsObjectId>,
    pub params: Vec<u8>,
}

/// Execution context provided to kernel modules
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct ExecutionContext {
    pub instruction: Instruction,
    pub objects: HashMap<UnitsObjectId, UnitsObject>,
    pub slot: u64,
    pub timestamp: u64,
}

/// Effect of kernel execution on a single object
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ObjectEffect {
    pub object_id: UnitsObjectId,
    pub before_image: Option<UnitsObject>,
    pub after_image: Option<UnitsObject>,
}

impl ObjectEffect {
    /// Create new object effect
    pub fn creation(object: UnitsObject) -> Self {
        Self {
            object_id: object.id,
            before_image: None,
            after_image: Some(object),
        }
    }
    
    /// Modify existing object effect
    pub fn modification(before: UnitsObject, after: UnitsObject) -> Self {
        Self {
            object_id: after.id,
            before_image: Some(before),
            after_image: Some(after),
        }
    }
    
    /// Delete object effect
    pub fn deletion(object: UnitsObject) -> Self {
        Self {
            object_id: object.id,
            before_image: Some(object),
            after_image: None,
        }
    }
}

/// Kernel error types
#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum KernelError {
    InvalidFunction = -1,
    InvalidParams = -2,
    InsufficientBalance = -3,
    Unauthorized = -4,
    TokenFrozen = -5,
    Overflow = -6,
    ObjectNotFound = -7,
    InvalidData = -8,
    IOError = -9,
    Panic = -10,
}

/// Trait that all kernel modules must implement
pub trait KernelModule {
    /// Execute the kernel module with the given context
    fn execute(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError>;
}

// System calls for no_std environment
#[cfg(not(feature = "std"))]
mod syscalls {
    extern "C" {
        fn sys_read(fd: i32, buf: *mut u8, count: usize) -> isize;
        fn sys_write(fd: i32, buf: *const u8, count: usize) -> isize;
        fn sys_exit(status: i32) -> !;
    }
    
    pub unsafe fn read(fd: i32, buf: &mut [u8]) -> Result<usize, ()> {
        let result = sys_read(fd, buf.as_mut_ptr(), buf.len());
        if result < 0 {
            Err(())
        } else {
            Ok(result as usize)
        }
    }
    
    pub unsafe fn write(fd: i32, buf: &[u8]) -> Result<usize, ()> {
        let result = sys_write(fd, buf.as_ptr(), buf.len());
        if result < 0 {
            Err(())
        } else {
            Ok(result as usize)
        }
    }
    
    pub unsafe fn exit(status: i32) -> ! {
        sys_exit(status)
    }
}

/// Read execution context from stdin
pub fn read_context() -> Result<ExecutionContext, KernelError> {
    #[cfg(not(feature = "std"))]
    {
        // In no_std environment, read from stdin using syscalls
        let mut size_buf = [0u8; 4];
        unsafe {
            syscalls::read(0, &mut size_buf).map_err(|_| KernelError::IOError)?;
        }
        let size = u32::from_le_bytes(size_buf) as usize;
        
        let mut data = alloc::vec![0u8; size];
        let mut read = 0;
        while read < size {
            let n = unsafe {
                syscalls::read(0, &mut data[read..]).map_err(|_| KernelError::IOError)?
            };
            if n == 0 {
                return Err(KernelError::IOError);
            }
            read += n;
        }
        
        borsh::from_slice(&data).map_err(|_| KernelError::InvalidData)
    }
    
    #[cfg(feature = "std")]
    {
        // In std environment, this would read from stdin
        // This is mainly for testing
        unimplemented!("read_context not implemented for std")
    }
}

/// Write effects to stdout
pub fn write_effects(effects: &[ObjectEffect]) -> Result<(), KernelError> {
    let data = borsh::to_vec(effects).map_err(|_| KernelError::InvalidData)?;
    let _size = (data.len() as u32).to_le_bytes();
    
    #[cfg(not(feature = "std"))]
    {
        unsafe {
            syscalls::write(1, &size).map_err(|_| KernelError::IOError)?;
            
            let mut written = 0;
            while written < data.len() {
                let n = syscalls::write(1, &data[written..]).map_err(|_| KernelError::IOError)?;
                if n == 0 {
                    return Err(KernelError::IOError);
                }
                written += n;
            }
        }
        Ok(())
    }
    
    #[cfg(feature = "std")]
    {
        // In std environment, this would write to stdout
        // This is mainly for testing
        unimplemented!("write_effects not implemented for std")
    }
}

/// Exit the program with a status code
pub fn exit(status: i32) -> ! {
    #[cfg(not(feature = "std"))]
    unsafe {
        syscalls::exit(status)
    }
    
    #[cfg(feature = "std")]
    std::process::exit(status)
}