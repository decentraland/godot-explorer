use tokio::sync::Semaphore;

pub trait SemaphoreExt {
    fn set_permits(&self, max: usize);
}

impl SemaphoreExt for Semaphore {
    fn set_permits(&self, max: usize) {
        let current_permits = self.available_permits();
        let permits_diff = max as i32 - current_permits as i32;

        if permits_diff > 0 {
            self.add_permits(permits_diff as usize);
        } else {
            self.forget_permits((-permits_diff) as usize);
        }
    }
}
