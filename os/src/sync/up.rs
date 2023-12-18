//! Uniprocessor interior mutability primitives
use core::cell::{RefCell, RefMut, Ref};

/// Wrap a static data structure inside it so that we are
/// able to access it without any `unsafe`.
///
/// We should only use it in uniprocessor.
///
/// In order to get mutable reference of inner data, call
/// `exclusive_access`.
pub struct UPSafeCell<T> {
    /// inner data
    inner: RefCell<T>,
}

unsafe impl<T> Sync for UPSafeCell<T> {}

impl<T> UPSafeCell<T> {
    /// User is responsible to guarantee that inner struct is only used in
    /// uniprocessor.
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }
    /// Panic if the data has been borrowed.
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
    pub fn nomut_access(&self) -> Ref<'_, T> {
        self.inner.borrow()
    }
}
pub struct UUPSafeCell<T> {
    /// inner data
    pub inner: RefCell<T>,
}

unsafe impl<T> Sync for UUPSafeCell<T> {}

impl<T> UUPSafeCell<T> {
    /// User is responsible to guarantee that inner struct is only used in
    /// uniprocessor.
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }
    /// Panic if the data has been borrowed.
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        trace!("borrow mut here");
        self.inner.borrow_mut()
    }

    pub fn nomut_access(&self) -> Ref<'_, T> {
        trace!("borrow here");
        self.inner.borrow()
    }

    pub fn unwrap(self) -> T {
        self.inner.into_inner()
    }
}
