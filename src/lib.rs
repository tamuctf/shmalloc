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

pub use libc::{
    MAP_32BIT, MAP_ANON, MAP_ANONYMOUS, MAP_DENYWRITE, MAP_EXECUTABLE, MAP_FILE, MAP_FIXED,
    MAP_FIXED_NOREPLACE, MAP_GROWSDOWN, MAP_HUGETLB, MAP_HUGE_16GB, MAP_HUGE_16MB, MAP_HUGE_1GB,
    MAP_HUGE_1MB, MAP_HUGE_256MB, MAP_HUGE_2GB, MAP_HUGE_2MB, MAP_HUGE_32MB, MAP_HUGE_512KB,
    MAP_HUGE_512MB, MAP_HUGE_64KB, MAP_HUGE_8MB, MAP_LOCKED, MAP_NONBLOCK, MAP_NORESERVE,
    MAP_POPULATE, MAP_PRIVATE, MAP_SHARED, MAP_SHARED_VALIDATE, MAP_STACK, MAP_SYNC,
};
pub use libc::{PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE};

/// Use shmalloc!
///
/// # Examples
///
/// Using a shared and anonymous map as a heap backing.
///
/// ```
/// use shmalloc::*;
///
/// // declare a heap with 2^24 bytes of memory available
/// #[global_allocator]
/// static ALLOCATOR: Shmeap<0, { 1 << 24 }, { PROT_READ | PROT_WRITE }, { MAP_ANONYMOUS | MAP_SHARED }, ""> = Shmeap::new();
/// ```
///
/// Using a file-backed heap (scary) which is executable (scarier) and located at 0x100000 (scariest).
///
/// ```
/// use shmalloc::*;
///
/// #[global_allocator]
/// static ALLOCATOR: Shmeap<0x100000, { 1 << 24 }, { PROT_READ | PROT_WRITE | PROT_EXEC }, { MAP_SHARED }, "file.heap"> = Shmeap::new();
/// ```
pub struct Shmeap<
    const BASE: usize,
    const SIZE: usize,
    const PROT: std::os::raw::c_int,
    const FLAGS: std::os::raw::c_int,
    const FILE: &'static str,
> {
    // SAFETY: mutex means it's thread-safe
    heap: Lazy<Mutex<Heap>>,
}

impl<
        const BASE: usize,
        const SIZE: usize,
        const PROT: std::os::raw::c_int,
        const FLAGS: std::os::raw::c_int,
        const FILE: &'static str,
    > Shmeap<BASE, SIZE, PROT, FLAGS, FILE>
{
    /// Instantiate a new shmeap
    pub const fn new() -> Shmeap<BASE, SIZE, PROT, FLAGS, FILE> {
        Shmeap {
            heap: Lazy::new(|| unsafe {
                let mut heap = Heap::empty();
                let map = match FILE {
                    "" => libc::mmap(BASE as _, SIZE, PROT, FLAGS, -1, 0),
                    path => {
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
        const FILE: &'static str,
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
