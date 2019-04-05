// For efficient thread-local strategies, we need to give each thread a
// numerical identifier. This small module encapsulates that.

use std::{
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

static THREAD_ID_CTR: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    pub static THREAD_ID: usize = THREAD_ID_CTR.fetch_add(1, Ordering::Relaxed);
}

#[derive(Clone, Copy)]
pub struct ThreadID {
    id: usize,
    _not_sendable_between_threads: PhantomData<*mut usize>,
}

impl ThreadID {
    pub fn load() -> Self {
        THREAD_ID.with(|&id| Self {
            id,
            _not_sendable_between_threads: PhantomData,
        })
    }
}

impl From<ThreadID> for usize {
    fn from(source: ThreadID) -> usize {
        source.id
    }
}
