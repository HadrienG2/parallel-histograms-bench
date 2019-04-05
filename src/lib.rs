pub mod impls;
pub mod thread_id;
pub mod traits;


#[cfg(test)]
mod tests {
    use rayon::prelude::*;
    use std::{
        sync::Mutex,
        time::Instant,
    };
    use crate::{
        impls::*,
        thread_id::*,
        traits::*,
    };

    // Parameters of the benchmarks are configured here
    const NUM_BINS: usize = 1000;
    const NUM_ROLLS: usize = 100_000_000;
    const BATCH_SIZE: usize = 100;
    const NUM_BUCKETS: usize = 2;

    // Generate a bunch of random numbers
    #[inline(never)]
    fn gen_input<'a>(rng: &mut impl rand::Rng, buf: &'a mut Vec<f32>) -> &'a [f32] {
        buf.clear();
        for _ in 0..BATCH_SIZE {
            buf.push(rng.gen())
        }
        &buf[..]
    }

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
        let mut buf = Vec::with_capacity(BATCH_SIZE);
        microbench(|| {
            for _ in 0..NUM_ROLLS / BATCH_SIZE {
                histogram.fill_with_id_mut(gen_input(&mut rng, &mut buf), id);
            }
            histogram.num_hits()
        })
    }

    fn parallel_microbench(histogram: impl SyncHistogram) {
        microbench(|| {
            (0..NUM_ROLLS / BATCH_SIZE)
                .into_par_iter()
                .for_each_init(
                    || (rand::thread_rng(), ThreadID::load(), Vec::with_capacity(BATCH_SIZE)),
                    |(rng, id, buf), _| histogram.fill_with_id(gen_input(rng, buf), *id)
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
