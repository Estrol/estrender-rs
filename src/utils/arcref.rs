use std::{cell::RefCell, sync::Arc};

#[cfg(any(debug_assertions, feature = "enable-release-validation"))]
use std::time::{Duration, Instant};

/// Custom ArcRef type that wraps a Arc with RefCell without
/// Making it difficult to type.
// #[derive(Clone)]
pub struct ArcRef<T> {
    inner: Arc<RefCell<T>>,
}

impl<T> ArcRef<T> {
    /// Create a new ArcRef with the given value.
    pub fn new(value: T) -> ArcRef<T> {
        ArcRef {
            inner: Arc::new(RefCell::new(value)),
        }
    }

    /// Creata a clone of the ArcRef. This will atomically increment the reference count.
    pub fn clone(&self) -> ArcRef<T> {
        ArcRef {
            inner: self.inner.clone(),
        }
    }

    /// Borrow the value immutably. This will atomically increment the reference count.
    /// If the value is already borrowed mutably, this will block until the mutable borrow is released.
    /// NOTE: In debug mode, this will panic if the value is already borrowed immutably for more than 5 seconds.
    pub fn wait_borrow(&self) -> std::cell::Ref<T> {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        let start = Instant::now();

        loop {
            if let Ok(borrow) = self.inner.try_borrow() {
                return borrow;
            }

            #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
            if start.elapsed() > Duration::from_secs(5) {
                panic!("wait_borrow: waited more than 5 seconds to acquire immutable borrow");
            }
        }
    }

    /// Borrow the value mutably. This will atomically increment the reference count.
    /// If the value is already borrowed, this will block until the borrow is released.
    /// NOTE: In debug mode, this will panic if the value is already borrowed mutably for more than 5 seconds.
    pub fn wait_borrow_mut(&self) -> std::cell::RefMut<T> {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        let start = Instant::now();

        loop {
            if let Ok(borrow) = self.inner.try_borrow_mut() {
                return borrow;
            }

            #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
            if start.elapsed() > Duration::from_secs(5) {
                panic!("wait_borrow_mut: waited more than 5 seconds to acquire mutable borrow");
            }
        }
    }

    /// Try to borrow the value immutably. This will atomically increment the reference count.
    /// Will panic if the value is already borrowed mutably.
    pub fn borrow(&self) -> std::cell::Ref<T> {
        self.inner.borrow()
    }

    /// Try to borrow the value mutably. This will atomically increment the reference count.
    /// Will panic if the value is already borrowed.
    pub fn borrow_mut(&self) -> std::cell::RefMut<T> {
        self.inner.borrow_mut()
    }

    /// Try to borrow the value immutably. This will atomically increment the reference count.
    /// Will return None if the value is already borrowed mutably.
    pub fn try_borrow(&self) -> Option<std::cell::Ref<T>> {
        self.inner.try_borrow().ok()
    }

    /// Try to borrow the value mutably. This will atomically increment the reference count.
    /// Will return None if the value is already borrowed.
    pub fn try_borrow_mut(&self) -> Option<std::cell::RefMut<T>> {
        self.inner.try_borrow_mut().ok()
    }

    /// Try to unwrap the ArcRef. This will return the inner value if there are no other references to it.
    /// If there are other references, this will return the ArcRef itself as an error.
    pub fn try_unwrap(self) -> Result<T, Self> {
        let refcelled = Arc::try_unwrap(self.inner).map_err(|arc| ArcRef { inner: arc })?;

        let inner = refcelled.into_inner();
        Ok(inner)
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn as_ptr(_self: &Self) -> *const T {
        Arc::as_ptr(&_self.inner) as *const T
    }
}

pub mod hasher {
    use super::ArcRef;
    use std::hash::Hash;
    use std::sync::Arc;

    impl<T: std::hash::Hash> Hash for ArcRef<T> {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            Arc::as_ptr(&self.inner).hash(state);
        }
    }
}

impl<T> std::fmt::Debug for ArcRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArcRef").finish()
    }
}

impl<T> Clone for ArcRef<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: PartialEq> PartialEq for ArcRef<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T: PartialEq> Eq for ArcRef<T> {}
