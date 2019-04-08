# Benchmark results on a LAL grid node

- Test hardware: Unknown Skylake (VM on dedicated CPU), 20 cores w/o HT, 2.2 GHz
- Compilation settings: As in Cargo.toml + RUSTFLAGS="-C target-cpu=native"

## Extremely pessimistic scenario (NUM_BINS=1, BATCH_SIZE=1, NUM_BUCKETS=1)

    test tests::parallel_atomic ... 25.191057657 ns/iter, ok
    test tests::parallel_mutex ... 187.025173793 ns/iter, ok
    test tests::parallel_thread_bucketized ... 235.164476573 ns/iter, ok
    test tests::parallel_thread_local ... 3.240601234 ns/iter, ok
    test tests::sequential_atomic ... 16.0762905 ns/iter, ok
    test tests::sequential_mutex ... 35.925994778 ns/iter, ok
    test tests::sequential_raw ... 11.010574201 ns/iter, ok
    test tests::sequential_thread_bucketized ... 38.131024918 ns/iter, ok
    test tests::sequential_thread_local ... 16.036592728 ns/iter, ok

## Extremely optimistic scenario (NUM_BINS=10000, BATCH_SIZE=10000, NUM_BUCKETS=20)

    test tests::parallel_atomic ... 3.605742097 ns/iter, ok
    test tests::parallel_mutex ... 7.258618561 ns/iter, ok
    test tests::parallel_thread_bucketized ... 0.480613646 ns/iter, ok
    test tests::parallel_thread_local ... 0.489055941 ns/iter, ok
    test tests::sequential_atomic ... 14.367838772 ns/iter, ok
    test tests::sequential_mutex ... 8.827962197 ns/iter, ok
    test tests::sequential_raw ... 8.074219191 ns/iter, ok
    test tests::sequential_thread_bucketized ... 8.631047287 ns/iter, ok
    test tests::sequential_thread_local ... 8.894028819 ns/iter, ok

## More realistic scenario (NUM_BINS=1000, BATCH_SIZE=100, NUM_BUCKETS=5)

    test tests::parallel_atomic ... 5.151963762 ns/iter, ok
    test tests::parallel_mutex ... 20.283309151 ns/iter, ok
    test tests::parallel_thread_bucketized ... 4.325697175 ns/iter, ok
    test tests::parallel_thread_local ... 0.810523986 ns/iter, ok
    test tests::sequential_atomic ... 13.71392596 ns/iter, ok
    test tests::sequential_mutex ... 9.033497823 ns/iter, ok
    test tests::sequential_raw ... 8.104193487 ns/iter, ok
    test tests::sequential_thread_bucketized ... 9.088121199 ns/iter, ok
    test tests::sequential_thread_local ... 9.130987813 ns/iter, ok

## ...with other amounts of buckets

### 2 buckets (= 10 cores per bucket)

    test tests::parallel_thread_bucketized ... 9.281130203 ns/iter, ok

### 4 buckets (= 5 cores per bucket)

    test tests::parallel_thread_bucketized ... 5.000326627 ns/iter, ok

### 10 buckets (= 2 cores per bucket)

    test tests::parallel_thread_bucketized ... 2.155673282 ns/iter, ok

## ...with larger batches (NUM_BINS=1000, BATCH_SIZE=1000, NUM_BUCKETS=5)

    test tests::parallel_atomic ... 5.188724898 ns/iter, ok
    test tests::parallel_mutex ... 9.942463333 ns/iter, ok
    test tests::parallel_thread_bucketized ... 2.071090486 ns/iter, ok
    test tests::parallel_thread_local ... 0.561970588 ns/iter, ok
    test tests::sequential_atomic ... 13.461204814 ns/iter, ok
    test tests::sequential_mutex ... 8.481398619 ns/iter, ok
    test tests::sequential_raw ... 7.780100246 ns/iter, ok
    test tests::sequential_thread_bucketized ... 8.573977355 ns/iter, ok
    test tests::sequential_thread_local ... 8.505542334 ns/iter, ok

## ...with smaller batches (NUM_BINS=1000, BATCH_SIZE=10, NUM_BUCKETS=5)

    test tests::parallel_atomic ... 5.093229756 ns/iter, ok
    test tests::parallel_mutex ... 65.031884841 ns/iter, ok
    test tests::parallel_thread_bucketized ... 13.615817525 ns/iter, ok
    test tests::parallel_thread_local ... 0.900930882 ns/iter, ok
    test tests::sequential_atomic ... 13.807840228 ns/iter, ok
    test tests::sequential_mutex ... 10.932945456 ns/iter, ok
    test tests::sequential_raw ... 8.803319916 ns/iter, ok
    test tests::sequential_thread_bucketized ... 11.876273384 ns/iter, ok
    test tests::sequential_thread_local ... 10.602798897 ns/iter, ok
