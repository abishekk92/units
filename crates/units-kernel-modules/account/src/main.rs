#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
use alloc::{collections::BTreeMap};

#[cfg(feature = "std")]
#[allow(unused_imports)]
use std::{collections::HashMap};

#[cfg(not(feature = "std"))]
units_kernel_sdk::use_default_allocator!();

#[allow(unused_imports)]
use account::module::AccountModule;
#[allow(unused_imports)]
use units_kernel_sdk::{
    ExecutionContext, ObjectEffect, KernelModule, KernelError,
};

/// Entry point for the kernel module  
#[cfg(not(feature = "std"))]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Read execution context from standard input
    let ctx = match read_context() {
        Ok(ctx) => ctx,
        Err(_) => units_kernel_sdk::exit(KernelError::InvalidParams as i32),
    };
    
    // Execute the module
    let effects = match AccountModule::execute(&ctx) {
        Ok(effects) => effects,
        Err(e) => units_kernel_sdk::exit(e as i32),
    };
    
    // Write effects to standard output
    match write_effects(&effects) {
        Ok(_) => units_kernel_sdk::exit(0),
        Err(_) => units_kernel_sdk::exit(KernelError::IOError as i32),
    }
}

/// Entry point for std builds (testing)
#[cfg(feature = "std")]
fn main() {
    println!("Account kernel module - std build for testing");
}

/// Panic handler for no_std environment
#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    units_kernel_sdk::exit(KernelError::Panic as i32)
}