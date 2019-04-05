use {
    crate::traits::SyncHistogram,
    std::sync::atomic::{AtomicUsize, Ordering},
};

// Thread-safe histogram that works by modifying buckets using atomic RMW ops
pub struct AtomicHistogram {
    bins: Vec<AtomicUsize>,
}

impl AtomicHistogram {
    pub fn new(num_bins: usize) -> Self {
        Self {
            bins: (0..num_bins).map(|_| AtomicUsize::new(0)).collect(),
        }
    }

    // In sequential mode, can go faster by using simple atomic load/store.
    // With a sufficiently smart compiler, performance should become identical
    // to that of the toy histogram.
    //
    // NOTE: Unfortunately, our Histogram impl cannot use this method because
    //       that would require specialization, and Rust doesn't have it yet...
    //
    pub fn fill_mut_fast(&mut self, values: &[f32]) {
        for value in values {
            let bin = (value * (self.bins.len() as f32)) as usize;
            let prev_bin = self.bins[bin].load(Ordering::Relaxed);
            self.bins[bin].store(prev_bin + 1, Ordering::Relaxed);
        }
    }
}

impl SyncHistogram for AtomicHistogram {
    fn fill(&self, values: &[f32]) {
        for value in values {
            let bin = (value * (self.bins.len() as f32)) as usize;
            self.bins[bin].fetch_add(1, Ordering::Relaxed);
        }
    }

    fn num_hits(&self) -> usize {
        self.bins.iter().map(|b| b.load(Ordering::Relaxed)).sum::<usize>()
    }
}