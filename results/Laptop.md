# Benchmark results on my laptop

- Test hardware: Intel(R) Core(TM) i7-4720HQ CPU @ 2.60GHz (4 cores + HT)
- Compilation settings: As in Cargo.toml + RUSTFLAGS="-C target-cpu=native"

## Extremely pessimistic scenario (NUM_BINS=1, BATCH_SIZE=1, NUM_BUCKETS=1)

    test tests::parallel_atomic ... 27.029199256666665 ns/iter, ok
    test tests::parallel_mutex ... 167.88396467666666 ns/iter, ok
    test tests::parallel_thread_bucketized ... 180.79664561666667 ns/iter, ok
    test tests::parallel_thread_local ... 4.124025323333333 ns/iter, ok
    test tests::sequential_atomic ... 14.031784113333334 ns/iter, ok
    test tests::sequential_mutex ... 26.161589016666667 ns/iter, ok
    test tests::sequential_raw ... 8.789889383333334 ns/iter, ok
    test tests::sequential_thread_bucketized ... 28.47075245 ns/iter, ok
    test tests::sequential_thread_local ... 12.84956274 ns/iter, ok

## Extremely optimistic scenario (NUM_BINS=10000, BATCH_SIZE=10000, NUM_BUCKETS=8)

    test tests::parallel_atomic ... 4.721201586666667 ns/iter, ok
    test tests::parallel_mutex ... 6.75420114 ns/iter, ok
    test tests::parallel_thread_bucketized ... 1.6864104633333334 ns/iter, ok
    test tests::parallel_thread_local ... 1.6842381166666667 ns/iter, ok
    test tests::sequential_atomic ... 10.23260938 ns/iter, ok
    test tests::sequential_mutex ... 6.63190593 ns/iter, ok
    test tests::sequential_raw ... 6.14889349 ns/iter, ok
    test tests::sequential_thread_bucketized ... 6.54847751 ns/iter, ok
    test tests::sequential_thread_local ... 6.610848116666666 ns/iter, ok

## More realistic scenario (NUM_BINS=1000, BATCH_SIZE=100, NUM_BUCKETS=2)

    test tests::parallel_atomic ... 7.664906616666666 ns/iter, ok
    test tests::parallel_mutex ... 14.599817486666666 ns/iter, ok
    test tests::parallel_thread_bucketized ... 7.489769783333333 ns/iter, ok
    test tests::parallel_thread_local ... 1.6810039266666668 ns/iter, ok
    test tests::sequential_atomic ... 10.22832624 ns/iter, ok
    test tests::sequential_mutex ... 7.0692014966666665 ns/iter, ok
    test tests::sequential_raw ... 6.351445623333333 ns/iter, ok
    test tests::sequential_thread_bucketized ... 7.45863165 ns/iter, ok
    test tests::sequential_thread_local ... 6.96611946 ns/iter, ok

## ...with more buckets (NUM_BINS=1000, BATCH_SIZE=100, NUM_BUCKETS=4)

    test tests::parallel_thread_bucketized ... 2.275929596666667 ns/iter, ok
    test tests::sequential_thread_bucketized ... 7.0084709033333334 ns/iter, ok

## ...with larger batches (NUM_BINS=1000, BATCH_SIZE=1000, NUM_BUCKETS=2)

    test tests::parallel_atomic ... 6.68549147 ns/iter, ok
    test tests::parallel_mutex ... 6.002736236666666 ns/iter, ok
    test tests::parallel_thread_bucketized ... 3.2388442766666667 ns/iter, ok
    test tests::parallel_thread_local ... 1.6367142533333334 ns/iter, ok
    test tests::sequential_atomic ... 10.093882856666667 ns/iter, ok
    test tests::sequential_mutex ... 6.560561 ns/iter, ok
    test tests::sequential_raw ... 6.093622053333333 ns/iter, ok
    test tests::sequential_thread_bucketized ... 6.568578303333333 ns/iter, ok
    test tests::sequential_thread_local ... 6.59893375 ns/iter, ok

## ...with smaller batches (NUM_BINS=1000, BATCH_SIZE=10, NUM_BUCKETS=2)

    test tests::parallel_atomic ... 5.983635763333333 ns/iter, ok
    test tests::parallel_mutex ... 34.87436967666667 ns/iter, ok
    test tests::parallel_thread_bucketized ... 16.687336846666668 ns/iter, ok
    test tests::parallel_thread_local ... 1.9348726633333333 ns/iter, ok
    test tests::sequential_atomic ... 10.142460523333334 ns/iter, ok
    test tests::sequential_mutex ... 8.494325803333334 ns/iter, ok
    test tests::sequential_raw ... 6.755554803333333 ns/iter, ok
    test tests::sequential_thread_bucketized ... 9.231934483333333 ns/iter, ok
    test tests::sequential_thread_local ... 8.063741403333333 ns/iter, ok
