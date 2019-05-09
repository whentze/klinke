use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    ptr,
    sync::atomic::{Ordering, AtomicUsize},
    thread,
};

use crossbeam_channel::{Sender, bounded};

// Each thread may hold a channel to send garbage on.
thread_local! {
    static DISPOSAL: Cell<Option<Sender<Garbage>>>
        = Cell::new(None);
}

// Garbage is a description of an allocation that should be cleaned up.
#[derive(Debug)]
struct Garbage {
    ptr: *mut u8,
    layout: Layout,
}

// We only send it around to deallocate, which is safe (the global allocator doesn't care).
unsafe impl Send for Garbage {}


// Threads that do hold such a channel are designated "audio thread".
fn am_audio_thread() -> bool {
    DISPOSAL.with(|c| {
        let o = c.take();
        let res = o.is_some();
        c.replace(o);
        res
    })
}

/// Declare that, from now on, this thread is regarded to be an "audio thread".
/// 
/// This will spawn a new clean-up thread that will take care of this thread's deallocations from now on
/// It also means that, to keep us honest, any allocations will fail while in debug mode.
pub(crate) fn become_audio_thread() {
    if !am_audio_thread() {
        DISPOSAL.with(|c| {
            let (tx, rx) = bounded::<Garbage>(256);
            thread::spawn(move || {
                while let Ok(garb) = rx.recv() {
                    unsafe { System.dealloc(garb.ptr, garb.layout) };
                }
            });
            c.replace(Some(tx));
        });
    };
}

static LEAK_COUNTER : AtomicUsize = AtomicUsize::new(0);

pub(crate) struct ZealousAllocator;

unsafe impl GlobalAlloc for ZealousAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // While debugging, forbid all allocations on the audio thread.
        if cfg!(debug_assertions) && am_audio_thread() {
            // This keeps devs on their toes.
            ptr::null_mut()
        } else {
            System.alloc(layout)
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if am_audio_thread() {
            // We can't deallocate here, this is rt country.
            DISPOSAL.with(|c| {
                let s = c.take().unwrap();
                s.try_send(Garbage { ptr, layout }).unwrap_or_else(|_| {
                    // We can't send it either, the channel is full or broken.
                    // The least we can do is tell everyone we made a mess.
                    LEAK_COUNTER.fetch_add(layout.size(), Ordering::Relaxed);
                });
                c.replace(Some(s));
            });
        } else {
            System.dealloc(ptr, layout);
        }
    }
}