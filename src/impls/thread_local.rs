use {
    crate::{
        impls::AtomicHistogram,
        thread_id::ThreadID,
        traits::SyncHistogram,
    },
    std::cell::UnsafeCell,
};

// Thread-safe histogram implementation which works by maintaining one histogram
// per thread. Maximally scalable to many threads, but least memory efficient.
//
// Note that although an AtomicHistogram is needed in order to avoid undefined
// behaviour on concurrent fill() and num_hits() calls, this implementation only
// uses atomic loads and stores, which are free on current hardware. So barring
// compiler mis-optimization, filling this histogram should be as fast as
// filling a ToyHistogram sequentially.
//
pub struct ThreadLocalHistogram {
    buckets: Vec<UnsafeCell<AtomicHistogram>>,
}

impl ThreadLocalHistogram {
    pub fn new(num_bins: usize) -> Self {
        Self {
            buckets: (0..num_cpus::get()).map(|_| UnsafeCell::new(AtomicHistogram::new(num_bins))).collect(),
        }
    }

    fn bucket(&self, id: ThreadID) -> &mut AtomicHistogram {
        let bucket_ptr = self.buckets[usize::from(id) % self.buckets.len()].get();
        unsafe { &mut *bucket_ptr }
    }
}

impl SyncHistogram for ThreadLocalHistogram {
    fn fill(&self, values: &[f32]) {
        self.fill_with_id(values, ThreadID::load())
    }

    fn fill_with_id(&self, values: &[f32], id: ThreadID) {
        self.bucket(id).fill_mut_fast(values)
    }

    fn num_hits(&self) -> usize {
        self.buckets.iter()
            .map(|b| unsafe { <AtomicHistogram as SyncHistogram>::num_hits(&*b.get()) })
            .sum::<usize>()
    }
}

unsafe impl Send for ThreadLocalHistogram {}
unsafe impl Sync for ThreadLocalHistogram {}