# Microbenchmarking parallel histogramming strategies

## Introduction

Parallel histogramming is a nontrivial problem. You need to trade between
low CPU overhead, low memory overhead, and scalability to many threads, and the
more you try to get all three at the same time, the more you pay in code
complexity and reduced user ergonomics.

This code aims to get a rough comparison of various implementation strategies
in a reduced test scenario that is cheap to implement: a 1D histogram, whose
bins always follow a regularly spaced [0; 1[ axis, whose inputs have equal
weight, filled from a uniform random distribution.

Notice that the simplest benchmarks will be bottlenecked on random number
generation. This is actually a good thing, as it allows studying scalability
in parallel environments.

## Available implementations

- A basic thread-unsafe "ToyHistogram"
- The same histogram, locked using a mutex
- A histogram whose bins are atomic counters, incremented using RMW operations
- Keeping a thread-local histogram per thread and merging them eventually
- A hybrid "bucketized" strategy with less than one histogram per thread

## Available tuning parameters

- How many bins the histogram has (NUM_BINS)
    * Will affect contention if locking is performed at bin granularity
    * Note that the uniform distribution is a favorable case, but you can
      emulate the effect of less uniform distributions with less bins.
- How much data is inserted into histograms (NUM_ROLLS)
    * Should only affect total running time. Aim for a few seconds per benchmark
      in order to get reasonable reproducibility and amortize initialization.
- How many entries are inserted per histogram fill (BATCH_SIZE)
    * More entries allow amortizing histogram-wide locking overhead
    * Users are accustomed to inserting only one entry at a time, and making
      them insert multiple entries per fill will require discipline.
- Number of buckets (NUM_BUCKETS)
    * Only affects bucketized strategies, tunes compromise between scalability
      and memory usage

## Results

- Test hardware: Intel(R) Core(TM) i7-4720HQ CPU @ 2.60GHz (4 cores + HT)
- Compilation settings: As in Cargo.toml + RUSTFLAGS="-C target-cpu=native"

### Extremely pessimistic scenario (NUM_BINS=1, BATCH_SIZE=1, NUM_BUCKETS=1)

    test tests::parallel_atomic ... 27.029199256666665 ns/iter, ok
    test tests::parallel_mutex ... 167.88396467666666 ns/iter, ok
    test tests::parallel_thread_bucketized ... 180.79664561666667 ns/iter, ok
    test tests::parallel_thread_local ... 4.124025323333333 ns/iter, ok
    test tests::sequential_atomic ... 14.031784113333334 ns/iter, ok
    test tests::sequential_mutex ... 26.161589016666667 ns/iter, ok
    test tests::sequential_raw ... 8.789889383333334 ns/iter, ok
    test tests::sequential_thread_bucketized ... 28.47075245 ns/iter, ok
    test tests::sequential_thread_local ... 12.84956274 ns/iter, ok

### Extremely optimistic scenario (NUM_BINS=10000, BATCH_SIZE=10000, NUM_BUCKETS=8)

    test tests::parallel_atomic ... 4.721201586666667 ns/iter, ok
    test tests::parallel_mutex ... 6.75420114 ns/iter, ok
    test tests::parallel_thread_bucketized ... 1.6864104633333334 ns/iter, ok
    test tests::parallel_thread_local ... 1.6842381166666667 ns/iter, ok
    test tests::sequential_atomic ... 10.23260938 ns/iter, ok
    test tests::sequential_mutex ... 6.63190593 ns/iter, ok
    test tests::sequential_raw ... 6.14889349 ns/iter, ok
    test tests::sequential_thread_bucketized ... 6.54847751 ns/iter, ok
    test tests::sequential_thread_local ... 6.610848116666666 ns/iter, ok

### More realistic scenario (NUM_BINS=1000, BATCH_SIZE=100, NUM_BUCKETS=2)

    test tests::parallel_atomic ... 7.664906616666666 ns/iter, ok
    test tests::parallel_mutex ... 14.599817486666666 ns/iter, ok
    test tests::parallel_thread_bucketized ... 7.489769783333333 ns/iter, ok
    test tests::parallel_thread_local ... 1.6810039266666668 ns/iter, ok
    test tests::sequential_atomic ... 10.22832624 ns/iter, ok
    test tests::sequential_mutex ... 7.0692014966666665 ns/iter, ok
    test tests::sequential_raw ... 6.351445623333333 ns/iter, ok
    test tests::sequential_thread_bucketized ... 7.45863165 ns/iter, ok
    test tests::sequential_thread_local ... 6.96611946 ns/iter, ok

### ...with more buckets (NUM_BINS=1000, BATCH_SIZE=100, NUM_BUCKETS=4)

    test tests::parallel_thread_bucketized ... 2.275929596666667 ns/iter, ok
    test tests::sequential_thread_bucketized ... 7.0084709033333334 ns/iter, ok

### ...with larger batches (NUM_BINS=1000, BATCH_SIZE=1000, NUM_BUCKETS=2)

    test tests::parallel_atomic ... 6.68549147 ns/iter, ok
    test tests::parallel_mutex ... 6.002736236666666 ns/iter, ok
    test tests::parallel_thread_bucketized ... 3.2388442766666667 ns/iter, ok
    test tests::parallel_thread_local ... 1.6367142533333334 ns/iter, ok
    test tests::sequential_atomic ... 10.093882856666667 ns/iter, ok
    test tests::sequential_mutex ... 6.560561 ns/iter, ok
    test tests::sequential_raw ... 6.093622053333333 ns/iter, ok
    test tests::sequential_thread_bucketized ... 6.568578303333333 ns/iter, ok
    test tests::sequential_thread_local ... 6.59893375 ns/iter, ok

### ...with smaller batches (NUM_BINS=1000, BATCH_SIZE=10, NUM_BUCKETS=2)

    test tests::parallel_atomic ... 5.983635763333333 ns/iter, ok
    test tests::parallel_mutex ... 34.87436967666667 ns/iter, ok
    test tests::parallel_thread_bucketized ... 16.687336846666668 ns/iter, ok
    test tests::parallel_thread_local ... 1.9348726633333333 ns/iter, ok
    test tests::sequential_atomic ... 10.142460523333334 ns/iter, ok
    test tests::sequential_mutex ... 8.494325803333334 ns/iter, ok
    test tests::sequential_raw ... 6.755554803333333 ns/iter, ok
    test tests::sequential_thread_bucketized ... 9.231934483333333 ns/iter, ok
    test tests::sequential_thread_local ... 8.063741403333333 ns/iter, ok

## Tentative conclusions

There is always a cost to synchronization. But the cost and scalability
characteristics depend on the synchronization mechanism in use.

### Mutexes

Mutexes have a rather high upfront cost, that can be amortized by inserting
many data points at once. This is the approach which ROOT 7 is currently
targeting. It should be noted, however, that this approach assumes sufficient
user education and suitable use cases, so its real-world applicability is
uncertain at this point in time.

Mutexes also deal very badly with lock contention, as shown in the worst case
scenario where the parallel case ends up a whopping 20x slower than the
sequential case. Again, this can be resolved via batching, which allows threads
to go to sleep and stop hammering the lock. Performance then becomes similar
to the sequential case.

Larger batches (~1000 points) are necessary to fully amortize the performance
hit introduced by the use of mutexes.

### Atomics

Atomics are, overall, cheaper than mutexes on individual transactions. They
cannot use batching optimizations, but they need it less than mutexes.

The performance of atomics is quite sensitive to the amount of bins in the
histogram (and, in real-world use cases, to the inhomogeneity of the input bin
distribution).

It is unclear how well atomics could scale to use of floating-point weights, as
there may not be a hardware fetch-add for this data type, requiring use of
compare-and-swap based emulation. The performance of this solution should be
studied.

To summarize, atomics beat mutexes in simple cases, but their less predictable
performance, inflexibility and lack of optimization headroom makes them hard to
recommend in more complex cases.

### Thread-local copies

This is the most scalable solution by a large margin, but it is obviously also
the most costly one in terms of memory usage. This can make it inappropriate for
scenarios where a huge number of histograms are in use, such as data quality
monitoring.

The performance difference between this thread-local implementation and
sequential usage of an unsychronized histogram is surprising. It may be that
the LLVM-based Rust compiler has issues optimizing out atomic load/stores here,
or it may be that these operations have an intrinsic impact on memory management
(e.g. forcing more flushes to RAM/caches) that is higher than expected.

### Bucketized copies

This was meant to be a midpoint between the mutex-based solution and the
thread-local solution, and I believe it does serve the intended purpose well.

If we look at parallel performance versus number of buckets at BATCH_SIZE=100,
comparing with the mutex implementation as an optimized case for 1 bucket and
the thread-local implementation as an optimized case for 8 buckets, we get this:

- Mutex: 14.6 ns/iter
- 1 bucket: 14.6 ns/iter
- 2 buckets: 7.5 ns/iter
- 4 buckets: 2.3 ns/iter
- 8 buckets: 1.8 ns/iter
- thread-local: 1.7ns/iter

The bucketized solution is obviously able to cover the continuum between a
single mutex-protected histogram and one local histogram per thread, allowing
one to fine-tune the memory usage vs scalability compromise and cover a broad
range of use cases without coming at a high code complexity cost.

In this sense, we may want to implement such a bucketized strategy on top of
ROOT, and/or to suggest its integration in ROOT 7.

Note that this benchmark's implementation implements a static mapping of threads
to buckets, which will only perform optimally in certain conditions (number of
threads is a multiple of the number of buckets, histogram load is well balanced
across threads). A dynamic bucket allocation strategy may be used to eliminate
these problems if needed, at a mild code complexity cost.

## Running the benchmarks yourself

This was developed using Rust 1.33. Compatibility with older Rust versions was
not checked and is likely not to reach very far in the past.

Tune the parameters in src/lib.rs as you like, then do...

    $ cargo test --release -- --nocapture --test-threads=1

When optimizing, you can focus on a single benchmark like this:

    $ cargo test --release sequential_raw -- --nocapture --test-threads=1

When profiling, you may want to force a benchmark build before to be sure that
you don't end up profiling a benchmark recompilation:

    $ cargo build --tests --release

## Why Rust?

Concurrent data structures can be hard to get right. Rust was specifically
designed to make this kind of job easier. Therefore, it is the perfect tool for
moving fast on this throwaway micro-project. Since the code is not to be kept
and reused, typical concerns of easy interfacing and build system integration
with C++ do not apply. So in the end, the only drawback is unfamiliarity, and I
believe that the benefits outweigh that cost.
