//! Shmalloc: a shmitty heap for shmitty code!
//!
//! Highly experimental. Do not use in production or anywhere.
//!
//! See [the whitepaper](https://isotropic.org/papers/chicken.pdf) for more details.

#![feature(allocator_api)]
#![feature(const_fn_fn_ptr_basics)]

use std::alloc::{GlobalAlloc, Layout};
use std::ptr::{self, NonNull};

use linked_list_allocator::Heap;
use once_cell::sync::Lazy;
use spin::Mutex;

/// Use shmalloc!
///
/// # Example
///
/// ```
/// use shmalloc::Shmeap;
///
/// // declare a heap with 2^24 bytes of memory available
/// #[global_allocator]
/// static ALLOCATOR: Shmeap<{ 1 << 24 }> = Shmeap::new();
/// ```
pub struct Shmeap<const SIZE: usize> {
    // SAFETY: mutex means it's thread-safe
    heap: Lazy<Mutex<Heap>>,
}

impl<const SIZE: usize> Shmeap<SIZE> {
    /// Instantiate a new shmeap
    pub const fn new() -> Shmeap<SIZE> {
        Shmeap {
            heap: Lazy::new(|| unsafe {
                let mut heap = Heap::empty();
                let map = libc::mmap(
                    ptr::null_mut(),
                    SIZE,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_ANONYMOUS | libc::MAP_SHARED,
                    -1,
                    0,
                );
                heap.init(map as usize, SIZE);
                Mutex::new(heap)
            }),
        }
    }

    /// Check how many bytes are currently in use by the heap
    pub fn used(&self) -> usize {
        self.heap.lock().used()
    }

    /// Check how many bytes are free in the heap
    pub fn free(&self) -> usize {
        self.heap.lock().free()
    }
}

unsafe impl<const SIZE: usize> GlobalAlloc for Shmeap<SIZE> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.heap
            .lock()
            .allocate_first_fit(layout)
            .ok()
            .map_or(ptr::null_mut(), |allocation| allocation.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.heap
            .lock()
            .deallocate(NonNull::new_unchecked(ptr), layout)
    }
}
