use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Default heap configuration for kernel modules
pub const DEFAULT_HEAP_START: usize = 0x80100000;
pub const DEFAULT_HEAP_SIZE: usize = 64 * 1024 * 1024; // 64MB

/// Simple bump allocator for kernel modules
/// 
/// This allocator provides fast allocation with minimal overhead
/// for the sandboxed RISC-V kernel module environment.
/// It never deallocates memory, which is suitable for short-lived
/// kernel module execution.
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: AtomicUsize,
}

impl BumpAllocator {
    /// Create a new bump allocator with default heap configuration
    pub const fn new() -> Self {
        Self::with_heap_range(DEFAULT_HEAP_START, DEFAULT_HEAP_SIZE)
    }
    
    /// Create a new bump allocator with custom heap range
    pub const fn with_heap_range(heap_start: usize, heap_size: usize) -> Self {
        Self {
            heap_start,
            heap_end: heap_start + heap_size,
            next: AtomicUsize::new(heap_start),
        }
    }
    
    /// Get the total heap size
    pub const fn heap_size(&self) -> usize {
        self.heap_end - self.heap_start
    }
    
    /// Get the amount of allocated memory
    pub fn allocated(&self) -> usize {
        self.next.load(Ordering::Relaxed) - self.heap_start
    }
    
    /// Get the amount of remaining memory
    pub fn remaining(&self) -> usize {
        self.heap_end - self.next.load(Ordering::Relaxed)
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let current = self.next.load(Ordering::Relaxed);
        let alloc_start = align_up(current, layout.align());
        let alloc_end = alloc_start + layout.size();
        
        if alloc_end > self.heap_end {
            // Out of memory
            core::ptr::null_mut()
        } else {
            // Try to update next pointer atomically
            match self.next.compare_exchange_weak(
                current,
                alloc_end,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => alloc_start as *mut u8,
                Err(_) => {
                    // Another thread allocated, retry
                    self.alloc(layout)
                }
            }
        }
    }
    
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't support deallocation
        // This is intentional for simplicity and performance
    }
}

// Unsafe implementation required for static initialization
unsafe impl Sync for BumpAllocator {}

/// Align address up to the given alignment
const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// Default global allocator instance
/// 
/// This can be used directly by kernel modules by importing this crate
/// and using the `use_default_allocator!` macro.
pub static DEFAULT_ALLOCATOR: BumpAllocator = BumpAllocator::new();

/// Macro to set up the default allocator and error handler for kernel modules
/// 
/// This macro should be called once in the main.rs of each kernel module.
/// It sets up the global allocator and allocation error handler, removing
/// the need for kernel module authors to write unsafe code.
/// 
/// # Example
/// 
/// ```rust
/// #![no_std]
/// #![no_main]
/// 
/// use units_kernel_sdk::use_default_allocator;
/// 
/// use_default_allocator!();
/// 
/// // Rest of your kernel module code...
/// ```
#[macro_export]
macro_rules! use_default_allocator {
    () => {
        #[global_allocator]
        static ALLOCATOR: &$crate::allocator::BumpAllocator = &$crate::allocator::DEFAULT_ALLOCATOR;
        
        #[alloc_error_handler]
        fn alloc_error_handler(_layout: core::alloc::Layout) -> ! {
            $crate::exit(-1)
        }
    };
}

/// Macro to set up a custom allocator for kernel modules
/// 
/// This allows kernel modules to use custom heap configurations if needed.
/// 
/// # Example
/// 
/// ```rust
/// use units_kernel_sdk::use_custom_allocator;
/// 
/// use_custom_allocator!(0x80200000, 32 * 1024 * 1024); // 32MB heap at different address
/// ```
#[macro_export]
macro_rules! use_custom_allocator {
    ($heap_start:expr, $heap_size:expr) => {
        static ALLOCATOR: $crate::allocator::BumpAllocator = 
            $crate::allocator::BumpAllocator::with_heap_range($heap_start, $heap_size);
        
        #[global_allocator]
        static GLOBAL_ALLOC: &$crate::allocator::BumpAllocator = &ALLOCATOR;
        
        #[alloc_error_handler]
        fn alloc_error_handler(_layout: core::alloc::Layout) -> ! {
            $crate::exit(-1)
        }
    };
}