pub(crate) mod thread_id;
pub(crate) mod traits;

use {
    crate::{
        thread_id::ThreadID,
        traits::{Histogram, SyncHistogram},
    },
    std::{
        cell::UnsafeCell,
        ops::DerefMut,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Mutex,
        },
    },
};

// Simplest histogram that we can do performance tests with
// One dimensional, everything has same weight, bin absciss in [0, 1[ range
struct ToyHistogram {
    bins: Vec<usize>,
}

impl ToyHistogram {
    fn new(num_bins: usize) -> Self {
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

// Basic thread-safe implementation that simply serializes using a lock
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
struct ThreadBucketizedHistogram {
    buckets: Vec<Mutex<ToyHistogram>>,
}

impl ThreadBucketizedHistogram {
    fn new(num_bins: usize, num_buckets: usize) -> Self {
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
struct ThreadLocalHistogram {
    buckets: Vec<UnsafeCell<ToyHistogram>>,
}

impl ThreadLocalHistogram {
    fn new(num_bins: usize) -> Self {
        Self {
            buckets: (0..num_cpus::get()).map(|_| UnsafeCell::new(ToyHistogram::new(num_bins))).collect(),
        }
    }

    fn get_bucket(&self, id: ThreadID) -> impl DerefMut<Target=ToyHistogram> + '_ {
        let bucket_ptr = self.buckets[usize::from(id) % self.buckets.len()].get();
        unsafe { &mut *bucket_ptr }
    }
}

impl SyncHistogram for ThreadLocalHistogram {
    fn fill(&self, values: &[f32]) {
        self.fill_with_id(values, ThreadID::load())
    }

    fn fill_with_id(&self, values: &[f32], id: ThreadID) {
        self.get_bucket(id).fill_mut(values)
    }

    // BUG: This is actually NOT safe to call if other threads are filling concurrently
    //
    // To achieve this, we'd need to make the binds atomic and manipulate these using
    // atomic loads and stores.
    //
    // Note that atomic read-modify-write operations are NOT needed in this case.
    // 
    fn num_hits(&self) -> usize {
        self.buckets.iter()
            .map(|b| unsafe { (*b.get()).num_hits() })
            .sum::<usize>()
    }
}

unsafe impl Send for ThreadLocalHistogram {}
unsafe impl Sync for ThreadLocalHistogram {}

// Completely different approach where buckets are modified using atomic RMW ops
struct AtomicHistogram {
    bins: Vec<AtomicUsize>,
}

impl AtomicHistogram {
    fn new(num_bins: usize) -> Self {
        Self {
            bins: (0..num_bins).map(|_| AtomicUsize::new(0)).collect(),
        }
    }
}

impl SyncHistogram for AtomicHistogram {
    fn fill(&self, values: &[f32]) {
        for value in values {
            let bin = f32::floor(value * (self.bins.len() as f32)) as usize;
            self.bins[bin].fetch_add(1, Ordering::Relaxed);
        }
    }

    fn num_hits(&self) -> usize {
        self.bins.iter().map(|b| b.load(Ordering::Relaxed)).sum::<usize>()
    }
}


#[cfg(test)]
mod tests {
    use rand::Rng;
    use rayon::prelude::*;
    use std::{
        sync::Mutex,
        time::Instant,
    };
    use super::*;

    // Parameters of the benchmarks are configured here
    const NUM_BINS: usize = 100;
    const NUM_ROLLS: usize = 100_000_000;
    const BATCH_SIZE: usize = 32;
    const NUM_BUCKETS: usize = 8;

    // Run user-specified microbench, return number of nanosecs per iteration
    fn microbench(runner: impl FnOnce() -> usize) {
        let start = Instant::now();
        let num_hits = runner();
        let duration = start.elapsed();
        assert_eq!(num_hits, NUM_ROLLS);

        let nanosecs = duration.as_secs() * 1_000_000_000
            + duration.subsec_nanos() as u64;
        let nanos_per_iter = (nanosecs as f64) / (NUM_ROLLS as f64);
        print!("{} ns/iter, ", nanos_per_iter);
    }

    fn sequential_microbench(mut histogram: impl Histogram) {
        let id = ThreadID::load();
        let mut rng = rand::thread_rng();
        microbench(|| {
            for _ in 0..NUM_ROLLS / BATCH_SIZE {
                histogram.fill_with_id_mut(&rng.gen::<[f32; BATCH_SIZE]>(), id);
            }
            histogram.num_hits()
        })
    }

    fn parallel_microbench(histogram: impl SyncHistogram) {
        microbench(|| {
            (0..NUM_ROLLS / BATCH_SIZE)
                .into_par_iter()
                .for_each_init(
                    || (rand::thread_rng(), ThreadID::load()),
                    |(rng, id), _| histogram.fill_with_id(&rng.gen::<[f32; BATCH_SIZE]>(), *id)
                );
            histogram.num_hits()
        })
    }

    #[test]
    fn sequential_raw() {
        let histogram = ToyHistogram::new(NUM_BINS);
        sequential_microbench(histogram)
    }

    #[test]
    fn sequential_atomic() {
        let histogram = AtomicHistogram::new(NUM_BINS);
        sequential_microbench(histogram)
    }

    #[test]
    fn sequential_mutex() {
        let histogram = Mutex::new(ToyHistogram::new(NUM_BINS));
        sequential_microbench(histogram)
    }

    #[test]
    fn sequential_thread_bucketized() {
        let histogram = ThreadBucketizedHistogram::new(NUM_BINS, NUM_BUCKETS);
        sequential_microbench(histogram)
    }

    #[test]
    fn sequential_thread_local() {
        let histogram = ThreadLocalHistogram::new(NUM_BINS);
        sequential_microbench(histogram)
    }

    #[test]
    fn parallel_atomic() {
        let histogram = AtomicHistogram::new(NUM_BINS);
        parallel_microbench(histogram)
    }

    #[test]
    fn parallel_mutex() {
        let histogram = Mutex::new(ToyHistogram::new(NUM_BINS));
        parallel_microbench(histogram)
    }

    #[test]
    fn parallel_thread_bucketized() {
        let histogram = ThreadBucketizedHistogram::new(NUM_BINS, NUM_BUCKETS);
        parallel_microbench(histogram)
    }

    #[test]
    fn parallel_thread_local() {
        let histogram = ThreadLocalHistogram::new(NUM_BINS);
        parallel_microbench(histogram)
    }
}
