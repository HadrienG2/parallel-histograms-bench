use crate::thread_id::ThreadID;

// Trait that any histogram must implement
//
// We're not trying to implement a real histogram library here, just
// microbenchmarking synchronization strategies, so it's okay to restrict
// ourselves to 1D histogram for the purpose of demonstration.
//
pub trait Histogram {
    // Insert a set of values into the histogram
    fn fill_mut(&mut self, values: &[f32]);

    // If the ID of the active thread is known, some implementations can use it
    // for optimization purposes by overriding this method
    fn fill_with_id_mut(&mut self, values: &[f32], _id: ThreadID) {
        self.fill_mut(values)
    }

    fn num_hits(&self) -> usize;
}

// Thread-safe version of Histogram that can be filled in parallel
pub trait SyncHistogram: Sync {
    fn fill(&self, values: &[f32]);

    fn fill_with_id(&self, values: &[f32], _id: ThreadID) {
        self.fill(values)
    }

    fn num_hits(&self) -> usize;
}

// Any thread-safe histogram can be used sequentially
impl<T: SyncHistogram> Histogram for T {
    fn fill_mut(&mut self, values: &[f32]) {
        self.fill(values)
    }

    fn fill_with_id_mut(&mut self, values: &[f32], id: ThreadID) {
        self.fill_with_id(values, id)
    }

    fn num_hits(&self) -> usize {
        <T as SyncHistogram>::num_hits(&self)
    }
}