//! Shmalloc: a shmitty heap for shmitty code!
//!
//! Highly experimental. Do not use in production or anywhere.
//!
//! See [the whitepaper](https://isotropic.org/papers/chicken.pdf) for more details.

#![feature(allocator_api)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(adt_const_params)]

use std::alloc::{GlobalAlloc, Layout};
use std::ffi::CString;
use std::ptr::{self, NonNull};

use linked_list_allocator::Heap;
use once_cell::sync::Lazy;
use spin::Mutex;

/// Use shmalloc!
///
/// # Examples
///
/// Using a shared and anonymous map as a heap backing.
///
/// ```
/// use shmalloc::Shmeap;
///
/// // declare a heap with 2^24 bytes of memory available
/// #[global_allocator]
/// static ALLOCATOR: Shmeap<0, { 1 << 24 }, { libc::PROT_READ | libc::PROT_WRITE }, { libc::MAP_ANONYMOUS | libc::MAP_SHARED }, None> = Shmeap::new();
/// ```
///
/// Using a file-backed heap (scary) which is executable (scarier) and located at 0x100000 (scariest).
///
/// ```
/// use shmalloc::Shmeap;
///
/// #[global_allocator]
/// static ALLOCATOR: Shmeap<0x100000, { 1 << 24 }, { libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC }, { libc::MAP_SHARED }, { Some("file.heap") }> = Shmeap::new();
/// ```
pub struct Shmeap<
    const BASE: usize,
    const SIZE: usize,
    const PROT: std::os::raw::c_int,
    const FLAGS: std::os::raw::c_int,
    const FILE: Option<&'static str>,
> {
    // SAFETY: mutex means it's thread-safe
    heap: Lazy<Mutex<Heap>>,
}

impl<
        const BASE: usize,
        const SIZE: usize,
        const PROT: std::os::raw::c_int,
        const FLAGS: std::os::raw::c_int,
        const FILE: Option<&'static str>,
    > Shmeap<BASE, SIZE, PROT, FLAGS, FILE>
{
    /// Instantiate a new shmeap
    pub const fn new() -> Shmeap<BASE, SIZE, PROT, FLAGS, FILE> {
        Shmeap {
            heap: Lazy::new(|| unsafe {
                let mut heap = Heap::empty();
                let map = match FILE {
                    None => libc::mmap(BASE as _, SIZE, PROT, FLAGS, -1, 0),
                    Some(path) => {
                        let cpath = CString::new(path).unwrap();
                        let file = libc::open(cpath.as_ptr(), PROT);
                        libc::mmap(BASE as _, SIZE, PROT, FLAGS, file, 0)
                    }
                };
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

unsafe impl<
        const BASE: usize,
        const SIZE: usize,
        const PROT: std::os::raw::c_int,
        const FLAGS: std::os::raw::c_int,
        const FILE: Option<&'static str>,
    > GlobalAlloc for Shmeap<BASE, SIZE, PROT, FLAGS, FILE>
{
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
