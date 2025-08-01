use std::sync::atomic::{AtomicU32, Ordering};

pub struct AtomicF32 {
    storage: AtomicU32,
}

impl AtomicF32 {
    pub const fn new() -> Self {
        Self { storage: AtomicU32::new(0) }
    }

    pub fn store(&self, value: f32) {
        self.storage.store(value.to_bits(), Ordering::Release)
    }

    pub fn load(&self) -> f32 {
        f32::from_bits(self.storage.load(Ordering::Acquire))
    }

    pub fn reset(&self) {
        self.storage.store(0, Ordering::Release);
    }
}
