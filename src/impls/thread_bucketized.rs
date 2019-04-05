use {
    crate::{
        impls::ToyHistogram,
        thread_id::ThreadID,
        traits::{Histogram, SyncHistogram},
    },
    std::{
        ops::DerefMut,
        sync::Mutex,
    },
};

// This is a compromise between Mutex<ToyHistogram> and ThreadLocalHistogram.
//
// We allow ourselves to maintain several copies of the histogram, but not one
// per thread. This is a very simple implementation where each thread is
// statically assigned to a given bucket, but it's good enough to show the
// overall performance characteristics of the approach as long as the number
// of buckets divides the number of threads evenly and the load is uniform.
//
// Notice that because buckets are shared between threads, a synchronization
// strategy is needed. Here, we use a simple mutex.
//
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