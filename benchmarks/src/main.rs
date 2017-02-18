extern crate libc;
extern crate x86;
extern crate rand;
extern crate treap;
extern crate teardown_tree;
extern crate splay;
//extern crate wio;

use bench_delete_range::{TeardownTreeMaster, TreapMaster, TreeBulk, TeardownTreeSingle, BTreeSetMaster, SplayMaster, IntervalTreeBulk, FilteredIntervalTreeBulk};
use bench_delete_range::{bench_refill_teardown_cycle, bench_refill, imptree_single_elem_range_n, btree_single_delete_n};

use std::time::Duration;

//#[cfg(all(feature = "unstable", target_os = "windows"))]
//fn set_affinity() {
//    assert!(wio::thread::Thread::current().unwrap().set_affinity_mask(8).is_ok());
//}
//
//#[cfg(not(target_os = "windows"))]
//fn set_affinity() {
//}


struct BenchJob<'a> {
    f: &'a Fn(usize, &'a [u64]) -> (String, Vec<u64>),
    spec: &'a [u64]
}

impl<'a> BenchJob<'a> {
    pub fn new(f: &'a Fn(usize, &'a [u64]) -> (String, Vec<u64>), spec: &'a [u64]) -> BenchJob<'a> {
        BenchJob { f: f, spec: spec }
    }
}


fn bench_table(batch_sz: usize, action: &str, jobs: &[BenchJob]) {
    println!("\n\t\t\t\t, , {}", action);
    print!("method\\N,\t\t\t");
    let ntimings = jobs[0].spec.len();
    let mut n = batch_sz;
    for _ in 0..ntimings {
        print!("{},\t", n);
        n *= 10;
    }
    println!();

    for job in jobs.iter() {
        let f = job.f;
        let spec = job.spec;

        let (descr, timings) = f(batch_sz, spec);

        print!("{},\t", descr);
        for time in timings.into_iter() {
            print!("{},\t", time);
        }
        println!();
    }
}




fn bench_teardown_full_impl<M: TeardownTreeMaster>(batch_sz: usize, spec: &[u64]) -> (String, Vec<u64>) {
    let mut n = batch_sz;
    let timings: Vec<u64> = spec.iter()
        .map(|iters| {
            let time = bench_refill_teardown_cycle::<M>(n, batch_sz, *iters);
            n *= 10;
            time
        })
        .collect();

    (M::descr_cycle(), timings)
}


fn bench_refill_impl<M: TeardownTreeMaster>(_: usize, spec: &[u64]) -> (String, Vec<u64>) {
    let mut n = 10;

    let timings: Vec<u64> = spec.iter()
        .map(|iters| {
            let time = bench_refill::<M>(n, *iters);
            n *= 10;
            time
        })
        .collect();

    (M::descr_cycle(), timings)
}


fn main() {
    bench_table(10, "Refill", &[
        BenchJob::new(&bench_refill_impl::<TreeBulk>,            &[170000000,   80000000,   12000000,   1100000,    65000,  2400,   230]),
        BenchJob::new(&bench_refill_impl::<IntervalTreeBulk>,    &[150000000,   70000000,   11000000,   1000000,    60000,  2200,   210]),
        BenchJob::new(&bench_refill_impl::<TreapMaster>,         &[7000000,     460000,     48000,      5000,       300,    25,     3]),
        BenchJob::new(&bench_refill_impl::<BTreeSetMaster>,      &[27000000,    3500000,    350000,     30000,      2300,   110,    10]),
        BenchJob::new(&bench_refill_impl::<SplayMaster>,         &[7000000,     540000,     50000,      4500,       400,    25,     3]),
    ]);


    bench_table(10, "Teardown in bulks of 10 items", &[
        BenchJob::new(&bench_teardown_full_impl::<TreeBulk>,         &[40000000, 3100000,    300000, 10000,  1200,   70, 7]),
    ]);

    bench_table(100, "Teardown in bulks of 100 items", &[
        BenchJob::new(&bench_teardown_full_impl::<TreeBulk>,            &[7000000, 700000, 70000,  4500,   400, 32]),
        BenchJob::new(&bench_teardown_full_impl::<IntervalTreeBulk>,    &[6000000, 350000, 35000,  2200,   200, 16]),
        BenchJob::new(&bench_teardown_full_impl::<TreapMaster>,         &[900000,  50000,  4000,   250,    20,  3]),
        BenchJob::new(&bench_teardown_full_impl::<BTreeSetMaster>,      &[1000000, 50000,  4000,   350,    30,  4]),
        BenchJob::new(&bench_teardown_full_impl::<SplayMaster>,         &[1000000, 50000,  4000,   350,    30,  4]),
        BenchJob::new(&bench_teardown_full_impl::<TeardownTreeSingle>,  &[2000000, 160000, 6000,   500,    50,  5]),
    ]);

    bench_table(1000, "Teardown in bulks of 1000 items", &[
        BenchJob::new(&bench_teardown_full_impl::<TreeBulk>,            &[700000, 80000,  8000,   700,    50]),
        BenchJob::new(&bench_teardown_full_impl::<IntervalTreeBulk>,    &[700000, 40000,  4000,   350,    25]),
        BenchJob::new(&bench_teardown_full_impl::<TreapMaster>,         &[50000,  5000,   400,    40,     3]),
        BenchJob::new(&bench_teardown_full_impl::<BTreeSetMaster>,      &[100000, 6000,   600,    40,     4]),
        BenchJob::new(&bench_teardown_full_impl::<SplayMaster>,         &[50000,  6000,   600,    40,     4]),
        BenchJob::new(&bench_teardown_full_impl::<TeardownTreeSingle>,  &[80000,  8000,   600,    60,     6]),
    ]);


    imptree_single_elem_range_n(100, 100,    200000);
    imptree_single_elem_range_n(1000, 100,   150000);
    imptree_single_elem_range_n(10000, 100,  100000);
    imptree_single_elem_range_n(100000, 100,  40000);
    imptree_single_elem_range_n(1000000, 100,  6000);

    btree_single_delete_n(100, 100,  30000);
    btree_single_delete_n(1000, 100,  5000);
    btree_single_delete_n(10000, 100, 1000);
    btree_single_delete_n(100000, 100,  80);
    btree_single_delete_n(1000000, 100, 30);


}



//---- unifying interfaces used in above benchmarks and its impls for 1) TeardownTree delete_range, 2) TeardownTree delete(), BTreeSet

mod bench_delete_range {
    use std::collections::BTreeSet;
    use std::ops::Range;
    use std::time;
    use std::iter::FromIterator;
    use std::fmt::{Formatter, Debug, Display, Result};
    use rand::{XorShiftRng, SeedableRng, Rng};
    use splay::SplaySet;

    use treap::TreapMap;
    use teardown_tree::{IntervalTeardownTreeSet, KeyInterval, Interval, TeardownTreeRefill, TeardownTreeSet, NoopFilter};
    use teardown_tree::util::make_teardown_seq;
    use teardown_tree::sink::{UncheckedVecRefSink};
    use super::{nanos, black_box};
    use super::ts::{Timestamp, new_timestamp, next_elapsed};

    pub type Tree = TeardownTreeSet<usize>;
    pub type TreeBulk = TeardownTreeBulk;


    pub fn btree_single_delete_n(n: usize, rm_items: usize, iters: u64) {
        let mut rng = XorShiftRng::from_seed([1,2,3,4]);
        let mut elapsed_cycles = 0;

        let start = time::Instant::now();
        for _ in 0..iters {
            let mut btset = BTreeSet::new();
            for i in 0..n {
                btset.insert(i);
            }

            let keys = {
                let mut keys = vec![];
                let mut pool: Vec<_> = (0..n).collect();

                for i in 0..rm_items {
                    let n = rng.gen_range(0, n - i);
                    let next = pool.swap_remove(n);
                    keys.push(next);
                }

                keys
            };

            let mut ts: Timestamp = new_timestamp();
            for i in 0..rm_items {
                let x = btset.remove(&keys[i]);
                black_box(x);
            }
            elapsed_cycles += next_elapsed(&mut ts);
        }
        let elapsed = start.elapsed();
        let elapsed_nanos = nanos(elapsed);

        let avg_cycles = elapsed_cycles/iters;
        println!("average time to delete {} random elements from BTreeMap using remove(), {} elements: {}cy, total: {}ms", rm_items, n, avg_cycles, elapsed_nanos/1000000)
    }

    pub fn imptree_single_elem_range_n(n: usize, rm_items: usize, iters: u64) {
        let mut rng = XorShiftRng::from_seed([1,2,3,4]);
        let mut elapsed_cycles = 0;

        let elems: Vec<_> = (1..n+1).collect();

        let tree = TeardownTreeBulk(Tree::new(elems));
        let mut copy = tree.clone();
        let mut output = Vec::with_capacity(tree.0.size());

        let start = time::Instant::now();
        for _ in 0..iters {
            let keys = {
                let mut pool: Vec<_> = (1..n+1).collect();
                let mut keys = vec![];

                for i in 0..rm_items {
                    let r = rng.gen_range(0, n-i);
                    let next = pool.swap_remove(r);
                    keys.push(next);
                }

                keys
            };

            copy.rfill(&tree);


            let mut ts: Timestamp = new_timestamp();
            for i in 0..rm_items {
                output.truncate(0);
                let x = copy.0.delete_range(keys[i]..keys[i]+1, UncheckedVecRefSink::new(&mut output));
                black_box(x);
            }
            elapsed_cycles += next_elapsed(&mut ts);
        }
        let elapsed = start.elapsed();
        let elapsed_nanos = nanos(elapsed);

        let avg_cycles = elapsed_cycles/iters;

        println!("average time to delete {} random elements from TeardownTree using delete_range(), {} elements: {}cy, total: {}ms", rm_items, n, avg_cycles, elapsed_nanos/1000000)
    }

    #[inline(never)]
    pub fn bench_refill<M: TeardownTreeMaster>(n: usize, iters: u64) -> u64 {
        let elems: Vec<_> = (0..n).collect();
        let tree = build::<M>(elems);
        let mut copy = tree.cpy();
        let mut elapsed_cycles = 0;

        let start = time::Instant::now();
        for _ in 0..iters {
            copy = black_box(copy);
            copy.clear();
            let mut ts: Timestamp = new_timestamp();
            copy.rfill(&tree);
            elapsed_cycles += next_elapsed(&mut ts);
        }
        let total = nanos(start.elapsed());

        let avg_cycles = elapsed_cycles/iters;
        println!("average time to refill {} with {} elements: {}cy, total: {}ms", M::descr_refill(), n, avg_cycles, total/1000000);
        avg_cycles
    }

    #[inline(never)]
    pub fn bench_refill_teardown_cycle<M: TeardownTreeMaster>(n: usize, rm_items: usize, iters: u64) -> u64 {
        let mut rng = XorShiftRng::from_seed([1,2,3,4]);
        let elems: Vec<_> = (0..n).collect();

        let ranges = make_teardown_seq(n, rm_items, &mut rng);

        let tree = build::<M>(elems);
        let mut copy = tree.cpy();
        let mut output = Vec::with_capacity(tree.sz());
        copy.del_range(0..n, &mut output);
        output.truncate(0);

        let start = time::Instant::now();
        let mut ts: Timestamp = new_timestamp();
        for iter in 0..iters {
            copy.rfill(&tree);
            for i in 0..ranges.len() {
                output.truncate(0);
                copy.del_range(ranges[i].clone(), &mut output);
                output = black_box(output);
                let expected_len = ranges[i].end - ranges[i].start;
                assert!(output.len() == expected_len, "range={:?}, expected: {}, len: {}, iter={}, i={}, output={:?}, copy={:?}, {}", ranges[i], expected_len, output.len(), iter, i, output, &copy, &copy);
            }
            assert!(copy.sz() == 0);
        }
        let elapsed_cycles = next_elapsed(&mut ts);
        let avg_cycles = elapsed_cycles/iters;
        let elapsed_nanos = nanos(start.elapsed());
        println!("average time to refill/tear down {}, {} elements in bulks of {} elements: {}cy, total: {}ms", M::descr_cycle(), n, rm_items, avg_cycles, elapsed_nanos/1000000);
        avg_cycles
    }


    fn build<M: TeardownTreeMaster>(mut elems: Vec<usize>) -> M {
        let mut rng = XorShiftRng::from_seed([42,142,1,7832]);

        // shuffle the elements, so that the tree comes out balanced
        for i in 0..elems.len() {
            let pos = rng.gen_range(i, elems.len());

            let tmp = elems[pos];
            elems[pos] = elems[i];
            elems[i] = tmp;
        }

        M::build(elems)
    }


    pub trait TeardownTreeMaster: Sized+Display {
        type Cpy: TeardownTreeCopy<Master = Self>;

        fn build(elems: Vec<usize>) -> Self;
        fn cpy(&self) -> Self::Cpy;
        fn sz(&self) -> usize;
        fn descr_cycle() -> String;
        fn descr_refill() -> String;
    }

    pub trait TeardownTreeCopy: Display+Debug {
        type Master: TeardownTreeMaster;
        type T: Debug;

        fn del_range(&mut self, range: Range<usize>, output: &mut Vec<Self::T>);
        #[inline(never)] fn rfill(&mut self, master: &Self::Master);
        fn sz(&self) -> usize;
        fn clear(&mut self);
        fn as_vec(&self) -> Vec<usize>;
    }


    /// for benchmarking TeardownTree delete_range()
    #[derive(Clone, Debug)]
    pub struct TeardownTreeBulk(TeardownTreeSet<usize>);

    impl TeardownTreeMaster for TeardownTreeBulk {
        type Cpy = TeardownTreeBulk;

        fn build(elems: Vec<usize>) -> Self {
            TeardownTreeBulk(TeardownTreeSet::new(elems))
        }

        fn cpy(&self) -> Self {
            self.clone()
        }

        fn sz(&self) -> usize {
            self.0.size()
        }

        fn descr_cycle() -> String {
            "TeardownTree::delete_range()".to_string()
        }

        fn descr_refill() -> String {
            "TeardownTree".to_string()
        }
    }

    impl TeardownTreeCopy for TeardownTreeBulk {
        type Master = TeardownTreeBulk;
        type T = usize;

        fn del_range(&mut self, range: Range<usize>, output: &mut Vec<usize>) {
            self.0.delete_range(range, UncheckedVecRefSink::new(output));
        }

        #[inline(never)]
        fn rfill(&mut self, master: &Self::Master) {
            self.0.refill(&master.0)
        }

        fn sz(&self) -> usize {
            self.0.size()
        }

        fn clear(&mut self) {
            self.0.clear();
        }

        fn as_vec(&self) -> Vec<usize> {
            self.0.iter().cloned().collect()
        }
    }

    impl Display for TeardownTreeBulk {
        fn fmt(&self, fmt: &mut Formatter) -> Result {
            Display::fmt(&self.0, fmt)
        }
    }


    /// for benchmarking TeardownTree delete()
    #[derive(Clone, Debug)]
    pub struct TeardownTreeSingle(TeardownTreeSet<usize>);

    impl TeardownTreeMaster for TeardownTreeSingle {
        type Cpy = TeardownTreeSingle;

        fn build(elems: Vec<usize>) -> Self {
            TeardownTreeSingle(TeardownTreeSet::new(elems))
        }

        fn cpy(&self) -> Self {
            self.clone()
        }

        fn sz(&self) -> usize {
            self.0.size()
        }

        fn descr_cycle() -> String {
            "TeardownTree::delete()".to_string()
        }

        fn descr_refill() -> String {
            "TeardownTree".to_string()
        }
    }

    impl TeardownTreeCopy for TeardownTreeSingle {
        type Master = TeardownTreeSingle;
        type T = usize;

        fn del_range(&mut self, range: Range<usize>, output: &mut Vec<usize>) {
            for i in range {
                if self.0.delete(&i) {
                    output.push(i);
                }
            }
        }

        #[inline(never)] 
        fn rfill(&mut self, master: &Self::Master) {
            self.0.refill(&master.0)
        }

        fn sz(&self) -> usize {
            self.0.size()
        }

        fn clear(&mut self) {
            self.0.clear();
        }

        fn as_vec(&self) -> Vec<usize> {
            self.0.iter().cloned().collect()
        }
    }

    impl Display for TeardownTreeSingle {
        fn fmt(&self, fmt: &mut Formatter) -> Result {
            Display::fmt(&self.0, fmt)
        }
    }


    #[derive(Debug)]
    pub struct BTreeSetMaster(BTreeSet<usize>);

    /// for benchmarking BTreeSet remove()
    impl TeardownTreeMaster for BTreeSetMaster {
        type Cpy = BTreeSetCopy;

        fn build(elems: Vec<usize>) -> Self {
            let mut set = BTreeSet::new();

            for elem in elems.into_iter() {
                set.insert(elem);
            }

            BTreeSetMaster(set)
        }

        fn cpy(&self) -> Self::Cpy {
            BTreeSetCopy { set: self.0.clone() }
        }

        fn sz(&self) -> usize {
            self.0.len()
        }

        fn descr_cycle() -> String {
            "BTreeSet::remove()".to_string()
        }

        fn descr_refill() -> String {
            "BTreeSet".to_string()
        }
    }

    #[derive(Debug)]
    pub struct BTreeSetCopy {
        set: BTreeSet<usize>
    }

    impl TeardownTreeCopy for BTreeSetCopy {
        type Master = BTreeSetMaster;
        type T = usize;

        fn del_range(&mut self, range: Range<usize>, output: &mut Vec<usize>) {
            for i in range {
                if self.set.remove(&i) {
                    output.push(i);
                }
            }
        }

        #[inline(never)]
        fn rfill(&mut self, master: &Self::Master) {
            assert!(self.set.is_empty(), "size={}", self.set.len());
            self.set = master.0.clone();
        }

        fn sz(&self) -> usize {
            self.set.len()
        }

        fn clear(&mut self) {
            self.set.clear();
        }

        fn as_vec(&self) -> Vec<usize> {
            self.set.iter().cloned().collect()
        }
    }

    impl Display for BTreeSetMaster {
        fn fmt(&self, fmt: &mut Formatter) -> Result {
            Debug::fmt(&self.0, fmt)
        }
    }

    impl Display for BTreeSetCopy {
        fn fmt(&self, fmt: &mut Formatter) -> Result {
            Debug::fmt(&self.set, fmt)
        }
    }


    //---- benchmarking Treap split/join ---------------------------------------------------------------
    pub struct TreapMaster(TreapMap<usize, ()>);

    pub struct TreapCopy(TreapMap<usize, ()>);

    impl TeardownTreeMaster for TreapMaster {
        type Cpy = TreapCopy;

        fn build(elems: Vec<usize>) -> Self {
            let iter = elems.into_iter().map(|x| (x, ()));
            TreapMaster(TreapMap::from_iter(iter))
        }

        fn cpy(&self) -> Self::Cpy {
            TreapCopy(self.0.clone())
        }

        fn sz(&self) -> usize {
            self.0.len()
        }

        fn descr_cycle() -> String {
            "Treap::delete_range".to_string()
        }

        fn descr_refill() -> String {
            "Treap".to_string()
        }
    }

    impl TeardownTreeCopy for TreapCopy {
        type Master = TreapMaster;
        type T = usize;

        fn del_range(&mut self, range: Range<usize>, output: &mut Vec<usize>) {
            self.0.remove_range(range, output);
        }

        #[inline(never)]
        fn rfill(&mut self, master: &Self::Master) {
            self.0 = master.0.clone()
        }

        fn sz(&self) -> usize {
            self.0.len()
        }

        fn clear(&mut self) {
            self.0.clear();
        }

        fn as_vec(&self) -> Vec<usize> {
            self.0.iter_ordered().map(|(&x, _)| x).collect()
        }
    }

    impl Display for TreapMaster {
        fn fmt(&self, _: &mut Formatter) -> Result {
            unimplemented!()
        }
    }

    impl Display for TreapCopy {
        fn fmt(&self, _: &mut Formatter) -> Result {
            unimplemented!()
        }
    }

    impl Debug for TreapCopy {
        fn fmt(&self, _: &mut Formatter) -> Result {
            unimplemented!()
        }
    }


    //---- benchmarking SplayTree split/join ---------------------------------------------------------------
    pub struct SplayMaster(SplaySet<usize>);

    pub struct SplayCopy(SplaySet<usize>);

    impl TeardownTreeMaster for SplayMaster {
        type Cpy = SplayCopy;

        fn build(elems: Vec<usize>) -> Self {
            SplayMaster(SplaySet::from_iter(elems.into_iter()))
        }

        fn cpy(&self) -> Self::Cpy {
            SplayCopy(self.0.clone())
        }

        fn sz(&self) -> usize {
            self.0.len()
        }

        fn descr_cycle() -> String {
            "SplayTree::remove_range()".to_string()
        }

        fn descr_refill() -> String {
            "SplayTree".to_string()
        }
    }

    impl TeardownTreeCopy for SplayCopy {
        type Master = SplayMaster;
        type T = usize;

        fn del_range(&mut self, range: Range<usize>, output: &mut Vec<usize>) {
            self.0.remove_range(&range.start .. &range.end, output);
        }

        #[inline(never)]
        fn rfill(&mut self, master: &Self::Master) {
            self.0 = master.0.clone()
        }

        fn sz(&self) -> usize {
            self.0.len()
        }

        fn clear(&mut self) {
            self.0.clear();
        }

        fn as_vec(&self) -> Vec<usize> {
            self.0.clone().into_iter().collect()
        }
    }

    impl Display for SplayMaster {
        fn fmt(&self, _: &mut Formatter) -> Result {
            unimplemented!()
        }
    }

    impl Display for SplayCopy {
        fn fmt(&self, _: &mut Formatter) -> Result {
            unimplemented!()
        }
    }

    impl Debug for SplayCopy {
        fn fmt(&self, _: &mut Formatter) -> Result {
            unimplemented!()
        }
    }


    /// for benchmarking IntervalTeardownTree delete_range()
    #[derive(Clone, Debug)]
    pub struct IntervalTreeBulk(IntervalTeardownTreeSet<KeyInterval<usize>>);

    impl TeardownTreeMaster for IntervalTreeBulk {
        type Cpy = IntervalTreeBulk;

        fn build(elems: Vec<usize>) -> Self {
            let elems = elems.into_iter().map(|x| KeyInterval::new(x, x)).collect();
            IntervalTreeBulk(IntervalTeardownTreeSet::new(elems))
        }

        fn cpy(&self) -> Self {
            self.clone()
        }

        fn sz(&self) -> usize {
            self.0.size()
        }

        fn descr_cycle() -> String {
            "IntervalTeardownTree::delete_range()".to_string()
        }

        fn descr_refill() -> String {
            "IntervalTeardownTree".to_string()
        }
    }

    impl TeardownTreeCopy for IntervalTreeBulk {
        type Master = IntervalTreeBulk;
        type T = KeyInterval<usize>;

        fn del_range(&mut self, range: Range<usize>, output: &mut Vec<Self::T>) {
            self.0.delete_overlap(&KeyInterval::new(range.start, range.end), UncheckedVecRefSink::new(output));
        }

        #[inline(never)]
        fn rfill(&mut self, master: &Self::Master) {
            self.0.refill(&master.0)
        }

        fn sz(&self) -> usize {
            self.0.size()
        }

        fn clear(&mut self) {
            self.0.clear();
        }

        fn as_vec(&self) -> Vec<usize> {
            self.0.iter().map(|it| it.a()).cloned().collect()
        }
    }


    impl Display for IntervalTreeBulk {
        fn fmt(&self, fmt: &mut Formatter) -> Result {
            Display::fmt(&self.0, fmt)
        }
    }

    /// for benchmarking IntervalTeardownTree::filter_range()
    #[derive(Clone, Debug)]
    pub struct FilteredIntervalTreeBulk(IntervalTeardownTreeSet<KeyInterval<usize>>);

    impl TeardownTreeMaster for FilteredIntervalTreeBulk {
        type Cpy = FilteredIntervalTreeBulk;

        fn build(elems: Vec<usize>) -> Self {
            let elems = elems.into_iter().map(|x| KeyInterval::new(x, x)).collect();
            FilteredIntervalTreeBulk(IntervalTeardownTreeSet::new(elems))
        }

        fn cpy(&self) -> Self {
            self.clone()
        }

        fn sz(&self) -> usize {
            self.0.size()
        }

        fn descr_cycle() -> String {
            "IntervalTeardownTree::filter_range()".to_string()
        }

        fn descr_refill() -> String {
            "IntervalTeardownTree".to_string()
        }
    }

    impl TeardownTreeCopy for FilteredIntervalTreeBulk {
        type Master = FilteredIntervalTreeBulk;
        type T = KeyInterval<usize>;

        fn del_range(&mut self, range: Range<usize>, output: &mut Vec<Self::T>) {
            self.0.filter_overlap(&KeyInterval::new(range.start, range.end), NoopFilter, UncheckedVecRefSink::new(output));
        }

        #[inline(never)]
        fn rfill(&mut self, master: &Self::Master) {
            self.0.refill(&master.0)
        }

        fn sz(&self) -> usize {
            self.0.size()
        }

        fn clear(&mut self) {
            self.0.clear();
        }

        fn as_vec(&self) -> Vec<usize> {
            self.0.iter().map(|it| it.a()).cloned().collect()
        }
    }


    impl Display for FilteredIntervalTreeBulk {
        fn fmt(&self, fmt: &mut Formatter) -> Result {
            Display::fmt(&self.0, fmt)
        }
    }
}



mod ts {
    use super::black_box;
    use x86::bits64::time::{rdtsc, rdtscp};

    pub type Timestamp = u64;

    #[inline]
    pub fn new_timestamp() -> Timestamp {
        // we cannot use rdtscp, it's bugged (some kind of memory or register corruption)

        // TODO: check whether a fence is really needed here. it sure is very expensive
//        unsafe { black_box(rdtsc()) }
        unsafe { rdtsc() }
    }

    #[inline]
    pub fn next_elapsed(prev_timestamp: &mut Timestamp) -> u64 {
        let timestamp = new_timestamp();
        let elapsed = timestamp - *prev_timestamp;
        *prev_timestamp = timestamp;
        elapsed
    }
}


pub fn black_box<T>(dummy: T) -> T {
    use std::ptr;
    use std::mem::forget;

    unsafe {
        let ret = ptr::read_volatile(&dummy as *const T);
        forget(dummy);
        ret
    }
}

#[inline]
fn nanos(d: Duration) -> u64 {
    d.as_secs()*1000000000 + d.subsec_nanos() as u64
}
