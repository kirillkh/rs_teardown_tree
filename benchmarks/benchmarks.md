*Benchmarks last updated for teardown_tree v0.6.5 / Rust 1.17.0-nightly.*


[Download the data][1].

To benchmark on your machine: ``cd rs_teardown_tree/benchmarks && cargo run --release``.

[1]: benchmarks.ods


TeardownTree vs other data structures
=====================================

**Refill/teardown**

In the benchmarks below, we initialize the master copy of the data structure, clone it and repeatedly:

1. Tear down the data structure by a random series of `delete_range` (or equivalent) operations.
1. Refill the copy so that its internal state is again equivalent to the master.
1. Rinse, repeat.

We measure the average time it takes to perform the steps above with two parameters: `N` (the number of items 
in the master copy) and `B` (number of items removed by a single `delete_range` operation). Each graph shows 
how much slower the given the operation was on average relative to `TeardownTree::delete_range()` (used as baseline).

![TeardownTree vs other data structures: full refill/teardown cycle in bulks of 10](ds_full_refill_teardown_10.svg?raw=true "full cycle/10")

![TeardownTree vs other data structures: full refill/teardown cycle in bulks of 100](ds_full_refill_teardown_100.svg?raw=true "full cycle/100")

![TeardownTree vs other data structures: full refill/teardown cycle in bulks of 1000](ds_full_refill_teardown_1000.svg?raw=true "full cycle/1000")

<br>
<br>
    
**Teardown**

The graphs below are based on the same data as above, except we subtract from each average time the time it 
takes to refill the data structure. This allows to compare the time it takes to tear down the data structure
separately from the time it takes to `refill` it.

![TeardownTree vs other data structures: teardown in bulks of 10](ds_teardown_10.svg?raw=true "teardown/10")

![TeardownTree vs other data structures: teardown in bulks of 100](ds_teardown_100.svg?raw=true "teardown/100")

![TeardownTree vs other data structures: teardown in bulks of 1000](ds_teardown_1000.svg?raw=true "teardown/1000")


TeardownTree variations
=====================================

We repeat the same benchmarks as above, but this time we compare 6 variations of the TeardownTree:

1. `TeardownSet::delete_range()`: the baseline. Each item stores a single `usize` value.
1. `TeardownMap::delete_range()`: same as above, but each item stores a key-value pair: `(usize, usize)`
1. `TeardownSet::filter_range()`: Each item stores a single `usize` value. The algorithm is a variation of `delete_range()`, modified to support filtering.
1. `IntervalTeardownSet::delete_overlap()`: each item stores an Interval with two `usize` bounds. Since we are interested in measuring the overhead of the algorithm over `TeardownSet::delete_range()`, both bounds are set to the same value, which makes the `IntervalTeardownSet` behave like a slower verison of `TeardownSet`.
1. `IntervalTeardownMap::delete_overlap()`: each item stores an Interval with two `usize` bounds and a `usize` value. 
1. `IntervalTeardownSet::filter_overlap()`: each item stores an Interval with two `usize` bounds. Supports filtering.

**Refill/teardown**

![TeardownTree variations: full refill/teardown cycle in bulks of 10](var_full_refill_teardown_10.svg?raw=true "full cycle/10")

![TeardownTree variations: full refill/teardown cycle in bulks of 100](var_full_refill_teardown_100.svg?raw=true "full cycle/100")

![TeardownTree variations: full refill/teardown cycle in bulks of 1000](var_full_refill_teardown_1000.svg?raw=true "full cycle/1000")

<br>
<br>
    
**Teardown**

![TeardownTree variations: teardown in bulks of 10](var_teardown_10.svg?raw=true "teardown/10")

![TeardownTree variations: teardown in bulks of 100](var_teardown_100.svg?raw=true "teardown/100")

![TeardownTree variations: teardown in bulks of 1000](var_teardown_1000.svg?raw=true "teardown/1000")
