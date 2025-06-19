use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct ArcRW<T> {
    inner: Arc<RwLock<T>>,
}

impl<T> ArcRW<T> {
    /// Create a new ArcRW with the given value.
    pub fn new(value: T) -> ArcRW<T> {
        ArcRW {
            inner: Arc::new(RwLock::new(value)),
        }
    }

    /// Clone the ArcRW, incrementing the reference count.
    pub fn clone(&self) -> ArcRW<T> {
        ArcRW {
            inner: self.inner.clone(),
        }
    }

    /// Try to borrow the value immutably.
    pub fn try_read(&self) -> Option<std::sync::RwLockReadGuard<T>> {
        self.inner.try_read().ok()
    }

    /// Try to borrow the value mutably.
    pub fn try_write(&self) -> Option<std::sync::RwLockWriteGuard<T>> {
        self.inner.try_write().ok()
    }

    /// Borrow the value immutably.
    pub fn read(&self) -> std::sync::RwLockReadGuard<T> {
        self.try_read().expect("Failed to acquire read lock")
    }

    /// Borrow the value mutably.
    pub fn write(&self) -> std::sync::RwLockWriteGuard<T> {
        self.try_write().expect("Failed to acquire write lock")
    }

    /// Wait for the read lock to be available, blocking until it can be acquired.
    /// [DEBUG] In debug mode, this will panic if the read lock is not acquired within 5 seconds.
    pub fn wait_read(&self) -> std::sync::RwLockReadGuard<T> {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        let now = std::time::Instant::now();
        loop {
            if let Ok(guard) = self.inner.try_read() {
                return guard;
            }

            #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
            if now.elapsed().as_secs() > 5 {
                panic!("wait_read: waited more than 5 seconds to acquire read lock");
            }
        }
    }

    /// Wait for the write lock to be available, blocking until it can be acquired.
    /// [DEBUG] In debug mode, this will panic if the write lock is not acquired within 5 seconds.
    pub fn wait_write(&self) -> std::sync::RwLockWriteGuard<T> {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        let now = std::time::Instant::now();
        loop {
            if let Ok(guard) = self.inner.try_write() {
                return guard;
            }

            #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
            if now.elapsed().as_secs() > 5 {
                panic!("wait_write: waited more than 5 seconds to acquire write lock");
            }
        }
    }

    /// Try to unwrap the ArcRW, returning the inner value if there are no other references.
    /// If there are other references, return an ArcRW with the inner value.
    pub fn into_inner(self) -> Result<T, Self> {
        let inner = Arc::try_unwrap(self.inner);
        match inner {
            Ok(lock) => match lock.into_inner() {
                Ok(value) => Ok(value),
                Err(poison_err) => Err(Self {
                    inner: Arc::new(RwLock::new(poison_err.into_inner())),
                }),
            },
            Err(val) => Err(Self { inner: val }),
        }
    }
}
