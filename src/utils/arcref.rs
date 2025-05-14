use std::{cell::RefCell, sync::Arc};

#[cfg(debug_assertions)]
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
        #[cfg(debug_assertions)]
        let start = Instant::now();

        loop {
            if let Ok(borrow) = self.inner.try_borrow() {
                return borrow;
            }

            #[cfg(debug_assertions)]
            if start.elapsed() > Duration::from_secs(5) {
                panic!("wait_borrow: waited more than 5 seconds to acquire immutable borrow");
            }
        }
    }

    /// Borrow the value mutably. This will atomically increment the reference count.
    /// If the value is already borrowed, this will block until the borrow is released.
    /// NOTE: In debug mode, this will panic if the value is already borrowed mutably for more than 5 seconds.
    pub fn wait_borrow_mut(&self) -> std::cell::RefMut<T> {
        #[cfg(debug_assertions)]
        let start = Instant::now();

        loop {
            if let Ok(borrow) = self.inner.try_borrow_mut() {
                return borrow;
            }

            #[cfg(debug_assertions)]
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
