//! What std provides a component, for a `no_std` component built for wasm32-wasip3: an allocator,
//! a panic handler, and threads.
//!
//! Without std, a component doesn't link the parts of wasi-libc std uses, and the imports they need,
//! e.g. `wasi:cli/stdin` and `wasi:cli/environment`. The allocator and threads are wasi-libc's,
//! the same std uses. Built for another target, e.g. the host for unit tests, std provides the
//! allocator and panic handler, and threads aren't supported.
//!
//! A component depends on this crate and links it with `use wasip3_rt as _;`, otherwise the
//! allocator and panic handler are missing.

#![no_std]

extern crate alloc;

#[cfg(target_env = "p3")]
use alloc::boxed::Box;

/// Run `f` on a new thread, detached.
///
/// On wasm32-wasip3 a thread is a component model cooperative thread, it runs when the threads
/// before it block or yield, e.g. a future that runs while a sync export, which has no task to spawn
/// it into, returns.
pub fn spawn<F: FnOnce() + 'static>(f: F) {
    #[cfg(target_env = "p3")]
    {
        extern "C" fn run(arg: *mut u8) -> *mut u8 {
            let f = unsafe { Box::from_raw(arg.cast::<Box<dyn FnOnce()>>()) };
            f();
            core::ptr::null_mut()
        }
        let f: Box<Box<dyn FnOnce()>> = Box::new(Box::new(f));
        let arg = Box::into_raw(f).cast();
        // created detached, `pthread_detach` of a thread that already exited joins it, a wait with a
        // timeout, which imports `wasi:clocks/monotonic-clock` and `wasi:clocks/system-clock`.
        // `pthread_attr_init` waits for a lock with a timeout too, the attributes are the defaults
        // `pthread_create` uses without attributes, read without a lock, as it does.
        let mut attr = libc::Attr::default();
        let mut thread = 0;
        let created = unsafe {
            libc::pthread_attr_setstacksize(&mut attr, libc::__default_stacksize as usize);
            libc::pthread_attr_setdetachstate(&mut attr, libc::PTHREAD_CREATE_DETACHED);
            libc::pthread_create(&mut thread, &attr, run, arg)
        };
        if created != 0 {
            drop(unsafe { Box::from_raw(arg.cast::<Box<dyn FnOnce()>>()) });
            panic!("failed to spawn thread");
        }
    }
    #[cfg(not(target_env = "p3"))]
    {
        drop(f);
        unimplemented!("threads need wasm32-wasip3");
    }
}

/// A trap, without std there is nowhere to write the message.
#[cfg(target_env = "p3")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

/// A trap, in place of wasi-libc's `exit`, which imports `wasi:cli/exit`.
///
/// A component isn't a process, there is nothing to exit, an instance that exits can't be used
/// again, like one that traps. wasi-libc calls it when the last thread exits, which doesn't happen,
/// the main thread is never on the thread list.
#[cfg(target_env = "p3")]
#[unsafe(no_mangle)]
extern "C" fn exit(_code: i32) -> ! {
    core::arch::wasm32::unreachable()
}

/// The lock of wasi-libc's thread list, a no-op, like wasi-libc's own.
///
/// wasi-libc's cooperative threads only define it with the thread-specific data of
/// `pthread_key_create`, linking it links them, and their destructors, which wait for a lock with a
/// timeout, importing `wasi:clocks/monotonic-clock` and `wasi:clocks/system-clock`. Nothing uses
/// thread-specific data, the release build optimizes them away, a debug build keeps the imports.
#[cfg(target_env = "p3")]
#[unsafe(no_mangle)]
extern "C" fn __tl_lock() {}

#[cfg(target_env = "p3")]
#[unsafe(no_mangle)]
extern "C" fn __tl_unlock() {}

#[cfg(target_env = "p3")]
#[global_allocator]
static ALLOCATOR: libc::Malloc = libc::Malloc;

#[cfg(target_env = "p3")]
mod libc {
    use core::alloc::{GlobalAlloc, Layout};

    /// pthread_t, a pointer
    pub type Thread = usize;

    /// pthread_attr_t, 36 bytes on wasm32, zeroed, with no guard, the only guard size WASI allows
    #[repr(C, align(8))]
    pub struct Attr([u8; 64]);

    impl Default for Attr {
        fn default() -> Self {
            Self([0; 64])
        }
    }

    pub const PTHREAD_CREATE_DETACHED: i32 = 1;

    #[link(name = "c")]
    unsafe extern "C" {
        fn malloc(size: usize) -> *mut u8;
        fn calloc(count: usize, size: usize) -> *mut u8;
        fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
        fn aligned_alloc(align: usize, size: usize) -> *mut u8;
        fn free(ptr: *mut u8);
        /// the stack size of a thread created without attributes
        pub static __default_stacksize: u32;
        pub fn pthread_attr_setstacksize(attr: *mut Attr, size: usize) -> i32;
        pub fn pthread_attr_setdetachstate(attr: *mut Attr, state: i32) -> i32;
        pub fn pthread_create(
            thread: *mut Thread,
            attr: *const Attr,
            start: extern "C" fn(*mut u8) -> *mut u8,
            arg: *mut u8,
        ) -> i32;
    }

    /// The alignment malloc guarantees, like std's allocator for wasm32
    const MIN_ALIGN: usize = 8;

    /// wasi-libc's allocator, the one std uses
    pub struct Malloc;

    unsafe impl GlobalAlloc for Malloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            unsafe {
                if layout.align() <= MIN_ALIGN && layout.align() <= layout.size() {
                    malloc(layout.size())
                } else {
                    aligned_alloc(layout.align(), layout.size())
                }
            }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            unsafe {
                if layout.align() <= MIN_ALIGN && layout.align() <= layout.size() {
                    calloc(layout.size(), 1)
                } else {
                    let ptr = self.alloc(layout);
                    if !ptr.is_null() {
                        ptr.write_bytes(0, layout.size());
                    }
                    ptr
                }
            }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
            unsafe { free(ptr) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            unsafe {
                if layout.align() <= MIN_ALIGN && layout.align() <= new_size {
                    realloc(ptr, new_size)
                } else {
                    let new = self.alloc(Layout::from_size_align_unchecked(new_size, layout.align()));
                    if !new.is_null() {
                        new.copy_from_nonoverlapping(ptr, layout.size().min(new_size));
                        free(ptr);
                    }
                    new
                }
            }
        }
    }
}
