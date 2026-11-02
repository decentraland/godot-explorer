use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Semaphore;

/// A semaphore wrapper that tracks the intended maximum permit count,
/// so `set_permits` works correctly even when permits are held by in-flight tasks.
pub struct CappedSemaphore {
    inner: Semaphore,
    max_permits: AtomicUsize,
}

impl CappedSemaphore {
    pub fn new(max: usize) -> Self {
        Self {
            inner: Semaphore::new(max),
            max_permits: AtomicUsize::new(max),
        }
    }

    /// Adjust the semaphore to a new intended maximum.
    ///
    /// The diff is computed against the stored max (not `available_permits`),
    /// so in-flight permits don't cause drift when they are later returned.
    pub fn set_permits(&self, new_max: usize) {
        let old_max = self.max_permits.swap(new_max, Ordering::SeqCst);
        if new_max > old_max {
            self.inner.add_permits(new_max - old_max);
        } else if old_max > new_max {
            self.inner.forget_permits(old_max - new_max);
        }
    }

    pub async fn acquire(&self) -> tokio::sync::SemaphorePermit<'_> {
        self.inner.acquire().await.unwrap()
    }
}
