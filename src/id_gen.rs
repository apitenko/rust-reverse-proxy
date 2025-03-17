use std::{cell::UnsafeCell, sync::Arc};

use atomic_counter::AtomicCounter;



/// Synchronized sequential ID generator
#[derive(Clone)]
pub struct IncrementalIdGeneratorAtomic {
    last_id: Arc<UnsafeCell<atomic_counter::RelaxedCounter>>,
}

unsafe impl Send for IncrementalIdGeneratorAtomic {}
unsafe impl Sync for IncrementalIdGeneratorAtomic {}

impl IncrementalIdGeneratorAtomic {
    pub fn new() -> IncrementalIdGeneratorAtomic {
        return IncrementalIdGeneratorAtomic {
            last_id: Arc::new(UnsafeCell::new(atomic_counter::RelaxedCounter::new(0))),
        };
    }

    pub fn next(&mut self) -> usize {
        unsafe {
            return (*self.last_id.get()).inc();
        }
    }
}
