use std::{
    cell::UnsafeCell,
    ops::DerefMut,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

// Trait that any histogram must implement
// For simplicity, we restrict ourselves to 1D histograms
trait Histogram {
    fn fill_all_mut(&mut self, values: &[f32]);
    fn fill_mut(&mut self, value: f32) { self.fill_all_mut(&[value]) }
    fn num_hits(&self) -> usize;
}

// Thread-safe version that can be filled in parallel
trait SyncHistogram: Sync {
    fn fill_all(&self, values: &[f32]);
    fn fill(&self, value: f32) { self.fill_all(&[value]) }
    fn num_hits(&self) -> usize;
}

// Any thread-safe histogram can be used sequentially
impl<T: SyncHistogram> Histogram for T {
    fn fill_all_mut(&mut self, values: &[f32]) { self.fill_all(values) }
    fn num_hits(&self) -> usize { self.num_hits() }
}

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
    fn fill_all_mut(&mut self, values: &[f32]) {
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
    fn fill_all(&self, values: &[f32]) {
        self.lock().unwrap().fill_all_mut(values)
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

    fn lock_bucket(&self) -> impl DerefMut<Target=ToyHistogram> + '_ {
        THREAD_ID.with(|id| self.buckets[id % self.buckets.len()].lock().unwrap())
    }
}

static THREAD_ID_CTR: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    pub static THREAD_ID: usize = THREAD_ID_CTR.fetch_add(1, Ordering::Relaxed);
}

impl SyncHistogram for ThreadBucketizedHistogram {
    fn fill_all(&self, values: &[f32]) {
        self.lock_bucket().fill_all_mut(values)
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

    fn get_bucket(&self) -> impl DerefMut<Target=ToyHistogram> + '_ {
        THREAD_ID.with(|id| {
            let bucket_ptr = self.buckets[id % self.buckets.len()].get();
            unsafe { &mut *bucket_ptr }
        })
    }
}

impl SyncHistogram for ThreadLocalHistogram {
    fn fill_all(&self, values: &[f32]) {
        self.get_bucket().fill_all_mut(values)
    }

    // WARNING: This is actually NOT safe to call if other threads are filling concurrently
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
    fn fill_all(&self, values: &[f32]) {
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
        microbench(|| {
            let mut rng = rand::thread_rng();
            for _ in 0..NUM_ROLLS / BATCH_SIZE {
                histogram.fill_all_mut(&rng.gen::<[f32; BATCH_SIZE]>());
            }
            histogram.num_hits()
        })
    }

    fn parallel_microbench(histogram: impl SyncHistogram) {
        microbench(|| {
            (0..NUM_ROLLS / BATCH_SIZE)
                .into_par_iter()
                .for_each_init(
                    || rand::thread_rng(),
                    |rng, _| histogram.fill_all(&rng.gen::<[f32; BATCH_SIZE]>())
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
