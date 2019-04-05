mod atomic;
mod thread_bucketized;
mod thread_local;

use {
    crate::traits::{Histogram, SyncHistogram},
    std::sync::Mutex,
};

pub use atomic::AtomicHistogram;
pub use thread_bucketized::ThreadBucketizedHistogram;
pub use thread_local::ThreadLocalHistogram;


// Toy histogram that's good enough for performance studies
// One dimensional, every input has same weight, bin absciss in [0, 1[ range.
//
// Every other implementation will attempt to provide similar behaviour in a
// multi-threaded filling environment.
//
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