mod atomic;

use {
    crate::{
        thread_id::ThreadID,
        traits::{Histogram, SyncHistogram},
    },
    std::{
        cell::UnsafeCell,
        ops::DerefMut,
        sync::Mutex,
    },
};

pub use atomic::AtomicHistogram;


// Toy histogram that's good enough for performance studies
// One dimensional, every input has same weight, bin absciss in [0, 1[ range.
// Every other implementation will mimick its behaviour
pub struct ToyHistogram {
    bins: Vec<usize>,
}

impl ToyHistogram {
    pub fn new(num_bins: usize) -> Self {
        Self {
            bins: vec![0; num_bins],
        }
    }
}

impl Histogram for ToyHistogram {
    fn fill_mut(&mut self, values: &[f32]) {
        for value in values {
            let bin = f32::floor(value * (self.bins.len() as f32)) as usize;
            self.bins[bin] += 1;
        }
    }

    fn num_hits(&self) -> usize {
        self.bins.iter().sum::<usize>()
    }
}

// A basic thread-safe implementation may be built via locking
impl SyncHistogram for Mutex<ToyHistogram> {
    fn fill(&self, values: &[f32]) {
        self.lock().unwrap().fill_mut(values)
    }

    fn num_hits(&self) -> usize {
        self.lock().unwrap().num_hits()
    }
}

// Slightly more advanced implementation which spreads accesses across a
// configurable number of buckets to get overhead of TLS down.
pub struct ThreadBucketizedHistogram {
    buckets: Vec<Mutex<ToyHistogram>>,
}

impl ThreadBucketizedHistogram {
    pub fn new(num_bins: usize, num_buckets: usize) -> Self {
        Self {
            buckets: (0..num_buckets).map(|_| Mutex::new(ToyHistogram::new(num_bins))).collect(),
        }
    }

    fn lock_bucket(&self, id: ThreadID) -> impl DerefMut<Target=ToyHistogram> + '_ {
        self.buckets[usize::from(id) % self.buckets.len()].lock().unwrap()
    }
}

impl SyncHistogram for ThreadBucketizedHistogram {
    fn fill(&self, values: &[f32]) {
        self.fill_with_id(values, ThreadID::load())
    }

    fn fill_with_id(&self, values: &[f32], id: ThreadID) {
        self.lock_bucket(id).fill_mut(values)
    }

    fn num_hits(&self) -> usize {
        self.buckets.iter()
            .map(|b| b.lock().unwrap().num_hits())
            .sum::<usize>()
    }
}

// More extreme cousin of ThreadBucketizedHistogram which assumes one bucket
// per thread and uses that for lock elision
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
        self.bucket(id).fill_mut_impl(values)
    }

    fn num_hits(&self) -> usize {
        self.buckets.iter()
            .map(|b| unsafe { <AtomicHistogram as SyncHistogram>::num_hits(&*b.get()) })
            .sum::<usize>()
    }
}

unsafe impl Send for ThreadLocalHistogram {}
unsafe impl Sync for ThreadLocalHistogram {}