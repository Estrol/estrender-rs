use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct ArcMut<T> {
    inner: Arc<Mutex<T>>,
}

impl<T> ArcMut<T> {
    pub fn new(value: T) -> ArcMut<T> {
        ArcMut {
            inner: Arc::new(Mutex::new(value)),
        }
    }

    pub fn clone(&self) -> ArcMut<T> {
        ArcMut {
            inner: self.inner.clone(),
        }
    }

    pub fn lock(&self) -> std::sync::MutexGuard<T> {
        self.inner.lock().unwrap()
    }

    pub fn try_lock(&self) -> Option<std::sync::MutexGuard<T>> {
        self.inner.try_lock().ok()
    }

    pub fn wait_borrow(&self) -> std::sync::MutexGuard<T> {
        #[cfg(debug_assertions)]
        let start = Instant::now();

        loop {
            if let Ok(borrow) = self.inner.try_lock() {
                return borrow;
            }

            #[cfg(debug_assertions)]
            if start.elapsed() > Duration::from_secs(5) {
                panic!("wait_borrow: waited more than 5 seconds to acquire borrow");
            }
        }
    }
}
