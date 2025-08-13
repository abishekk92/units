use core::alloc::{GlobalAlloc, Layout};

/// Simple bump allocator for kernel modules
struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let alloc_start = align_up(self.next, layout.align());
        let alloc_end = alloc_start + layout.size();

        if alloc_end > self.heap_end {
            // Out of memory
            core::ptr::null_mut()
        } else {
            self.next = alloc_end;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't support deallocation
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator {
    heap_start: 0x80100000, // Start of heap (after code/data)
    heap_end: 0x84000000,   // End of heap (64MB total)
    next: 0x80100000,
};

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

#[alloc_error_handler]
fn alloc_error_handler(_layout: Layout) -> ! {
    units_kernel_sdk::exit(-1)
}