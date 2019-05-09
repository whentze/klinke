use std::{cell::UnsafeCell, ops::Shl, ptr::NonNull};

/// An read-only f32 to be used as an input for a module.
#[derive(Debug)]
pub struct Input(Option<NonNull<f32>>);

unsafe impl Send for Input{}

impl Input {
    /// Read the value from this input.
    /// 
    /// Returns 0.0 if this input is not connected.
    /// If you need to differentiate the case of unconnected inputs, use `is_connected()`.
    pub fn get(&self) -> f32 {
        self.0.map(|n| unsafe { *n.as_ptr() }).unwrap_or(0.0)
    }

    /// Whether or not this input is currently connected.
    pub fn is_connected(&self) -> bool {
        self.0.is_some()
    }

    /// Connect this input to some other module's output.
    /// 
    /// If it is already connected, the old connection will be overwritten.
    /// 
    /// # Safety
    /// 
    /// You must ensure that `disconnect()` is called on this `Input` before
    /// `output` is moved, deallocated or otherwise deallocated.
    pub(crate) unsafe fn connect_to(&mut self, output: &mut Output) {
        self.0 = NonNull::new(output.0.get());
    }

    pub(crate) fn points_within(&self, bank: &[Output]) -> bool {
        match self.0 {
            Some(n) => unsafe { 
                let ptr = n.as_ptr();
                let begin = bank.as_ptr() as *mut f32;
                let end = bank.as_ptr().add(bank.len()) as *mut f32;
                ptr >= begin && ptr < end 
            },
            None => false,
        }
    }

    /// Disconnect this input from any output it may be connected to.
    /// This is always safe to call, regardless of whether `self` is currently connected.
    pub(crate) fn disconnect(&mut self) {
        self.0 = None;
    }
}

impl Default for Input {
    fn default() -> Self {
        Self(None)
    }
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct Output(UnsafeCell<f32>);

unsafe impl Send for Output{}

impl Output {
    pub fn set(&self, val: f32) {
        unsafe { self.0.get().write(val) };
    }
}

impl Shl<f32> for &Output {
    type Output = ();

    fn shl(self, rhs: f32) {
        self.set(rhs)
    }
}
