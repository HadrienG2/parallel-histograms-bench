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

- [On my laptop](results/Laptop.md)
- [On a LAL Grid node](results/Grid.md)

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

If we look at parallel performance versus number of buckets at BATCH_SIZE=100
on the 20-core LAL grid node, comparing it with the mutex implementation as an
optimized case for 1 bucket and the thread-local implementation as an optimized
case for 20 buckets, we get this:

    Mutex: 16.1 ns/iter
    1 bucket   aka 20 threads/bucket: 14.4 ns/iter
    2 buckets  aka 10 threads/bucket: 8.4 ns/iter
    4 buckets  aka 5  threads/bucket: 5.1 ns/iter
    5 buckets  aka 4  threads/bucket: 4.3 ns/iter
    10 buckets aka 2  threads/bucket: 2.0 ns/iter
    20 buckets aka 1  thread/bucket:  0.9 ns/iter
    thread-local: 0.9ns/iter

(It is not clear why the mutex solution is a bit slower than the single-bucket
solution, as they are algorithmically equivalent. One possibility is that the
extra computations needed to locate and access that single bucket reduce lock
contention by just a tiny bit)

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
