extern crate rand;
extern crate treap;
extern crate teardown_tree;
extern crate splay;
//extern crate wio;
extern crate cpuprofiler;


use bench_delete_range::{TreapMaster, TreeBulk, TeardownTreeSingle, BTreeSetMaster, SplayMaster, IntervalTreeBulk};
use bench_delete_range::{bench_refill_teardown_cycle, bench_refill, imptree_single_elem_range_n, btree_single_delete_n};

use std::time::Duration;

#[inline]
fn nanos(d: Duration) -> u64 {
    d.as_secs()*1000000000 + d.subsec_nanos() as u64
}

//#[cfg(all(feature = "unstable", target_os = "windows"))]
//fn set_affinity() {
//    assert!(wio::thread::Thread::current().unwrap().set_affinity_mask(8).is_ok());
//}
//
//#[cfg(not(target_os = "windows"))]
//fn set_affinity() {
//}

fn main() {
//    set_affinity();
    bench_refill_teardown_cycle::<TreeBulk>(1000000, 1000, 6000);
    return;

    bench_refill_teardown_cycle::<TreeBulk>(100, 100,    4500000);
    bench_refill_teardown_cycle::<TreeBulk>(1000, 100,    700000);
    bench_refill_teardown_cycle::<TreeBulk>(10000, 100,    70000);
    bench_refill_teardown_cycle::<TreeBulk>(100000, 100,    4500);
    bench_refill_teardown_cycle::<TreeBulk>(1000000, 100,    400);
    bench_refill_teardown_cycle::<TreeBulk>(10000000, 100,    32);

    bench_refill_teardown_cycle::<TreeBulk>(1000, 1000,  700000);
    bench_refill_teardown_cycle::<TreeBulk>(10000, 1000,  80000);
    bench_refill_teardown_cycle::<TreeBulk>(100000, 1000,  8000);
    bench_refill_teardown_cycle::<TreeBulk>(1000000, 1000,  700);
    bench_refill_teardown_cycle::<TreeBulk>(10000000, 1000,  50);


    bench_refill_teardown_cycle::<IntervalTreeBulk>(100, 100,    4500000);
    bench_refill_teardown_cycle::<IntervalTreeBulk>(1000, 100,    350000);
    bench_refill_teardown_cycle::<IntervalTreeBulk>(10000, 100,    35000);
    bench_refill_teardown_cycle::<IntervalTreeBulk>(100000, 100,    2200);
    bench_refill_teardown_cycle::<IntervalTreeBulk>(1000000, 100,    200);
    bench_refill_teardown_cycle::<IntervalTreeBulk>(10000000, 100,    16);

    bench_refill_teardown_cycle::<IntervalTreeBulk>(1000, 1000,  350000);
    bench_refill_teardown_cycle::<IntervalTreeBulk>(10000, 1000,  40000);
    bench_refill_teardown_cycle::<IntervalTreeBulk>(100000, 1000,  4000);
    bench_refill_teardown_cycle::<IntervalTreeBulk>(1000000, 1000,  350);
    bench_refill_teardown_cycle::<IntervalTreeBulk>(10000000, 1000,  25);


    bench_refill_teardown_cycle::<TreapMaster>(100, 100, 300000);
    bench_refill_teardown_cycle::<TreapMaster>(1000, 100, 25000);
    bench_refill_teardown_cycle::<TreapMaster>(10000, 100, 4000);
    bench_refill_teardown_cycle::<TreapMaster>(100000, 100, 250);
    bench_refill_teardown_cycle::<TreapMaster>(1000000, 100, 20);
    bench_refill_teardown_cycle::<TreapMaster>(10000000, 100, 3);

    bench_refill_teardown_cycle::<TreapMaster>(1000, 1000, 10000);
    bench_refill_teardown_cycle::<TreapMaster>(10000, 1000, 5000);
    bench_refill_teardown_cycle::<TreapMaster>(100000, 1000, 400);
    bench_refill_teardown_cycle::<TreapMaster>(1000000, 1000,  40);
    bench_refill_teardown_cycle::<TreapMaster>(10000000, 1000,  3);


    bench_refill_teardown_cycle::<BTreeSetMaster>(100, 100, 500000);
    bench_refill_teardown_cycle::<BTreeSetMaster>(1000, 100, 50000);
    bench_refill_teardown_cycle::<BTreeSetMaster>(10000, 100, 4000);
    bench_refill_teardown_cycle::<BTreeSetMaster>(100000, 100, 350);
    bench_refill_teardown_cycle::<BTreeSetMaster>(1000000, 100, 30);
    bench_refill_teardown_cycle::<BTreeSetMaster>(10000000, 100, 4);

    bench_refill_teardown_cycle::<BTreeSetMaster>(1000, 1000, 50000);
    bench_refill_teardown_cycle::<BTreeSetMaster>(10000, 1000, 6000);
    bench_refill_teardown_cycle::<BTreeSetMaster>(100000, 1000, 600);
    bench_refill_teardown_cycle::<BTreeSetMaster>(1000000, 1000, 40);
    bench_refill_teardown_cycle::<BTreeSetMaster>(10000000, 1000, 4);


    bench_refill_teardown_cycle::<SplayMaster>(100, 100, 500000);
    bench_refill_teardown_cycle::<SplayMaster>(1000, 100, 50000);
    bench_refill_teardown_cycle::<SplayMaster>(10000, 100, 4000);
    bench_refill_teardown_cycle::<SplayMaster>(100000, 100, 350);
    bench_refill_teardown_cycle::<SplayMaster>(1000000, 100, 30);
    bench_refill_teardown_cycle::<SplayMaster>(10000000, 100, 4);

    bench_refill_teardown_cycle::<SplayMaster>(1000, 1000, 50000);
    bench_refill_teardown_cycle::<SplayMaster>(10000, 1000, 6000);
    bench_refill_teardown_cycle::<SplayMaster>(100000, 1000, 600);
    bench_refill_teardown_cycle::<SplayMaster>(1000000, 1000, 40);
    bench_refill_teardown_cycle::<SplayMaster>(10000000, 1000, 4);

    bench_refill_teardown_cycle::<TeardownTreeSingle>(100, 100, 2000000);
    bench_refill_teardown_cycle::<TeardownTreeSingle>(1000, 100,  60000);
    bench_refill_teardown_cycle::<TeardownTreeSingle>(10000, 100,  6000);
    bench_refill_teardown_cycle::<TeardownTreeSingle>(100000, 100,  500);
    bench_refill_teardown_cycle::<TeardownTreeSingle>(1000000, 100,  50);
    bench_refill_teardown_cycle::<TeardownTreeSingle>(10000000, 100,  5);

    bench_refill_teardown_cycle::<TeardownTreeSingle>(1000, 1000,  80000);
    bench_refill_teardown_cycle::<TeardownTreeSingle>(10000, 1000,  8000);
    bench_refill_teardown_cycle::<TeardownTreeSingle>(100000, 1000,  600);
    bench_refill_teardown_cycle::<TeardownTreeSingle>(1000000, 1000,  60);
    bench_refill_teardown_cycle::<TeardownTreeSingle>(10000000, 1000,  6);



    bench_refill::<TreeBulk>(100, 40000000);
    bench_refill::<TreeBulk>(1000, 6000000);
    bench_refill::<TreeBulk>(10000, 500000);
    bench_refill::<TreeBulk>(100000, 40000);
    bench_refill::<TreeBulk>(1000000, 1200);
    bench_refill::<TreeBulk>(10000000, 110);


    bench_refill::<IntervalTreeBulk>(100, 40000000);
    bench_refill::<IntervalTreeBulk>(1000, 6000000);
    bench_refill::<IntervalTreeBulk>(10000, 500000);
    bench_refill::<IntervalTreeBulk>(100000, 40000);
    bench_refill::<IntervalTreeBulk>(1000000, 1200);
    bench_refill::<IntervalTreeBulk>(10000000, 110);


    bench_refill::<TreapMaster>(100, 260000);
    bench_refill::<TreapMaster>(1000, 28000);
    bench_refill::<TreapMaster>(10000, 3000);
    bench_refill::<TreapMaster>(100000, 220);
    bench_refill::<TreapMaster>(1000000, 25);
    bench_refill::<TreapMaster>(10000000, 3);


    bench_refill::<BTreeSetMaster>(100, 1700000);
    bench_refill::<BTreeSetMaster>(1000, 180000);
    bench_refill::<BTreeSetMaster>(10000, 16000);
    bench_refill::<BTreeSetMaster>(100000, 1300);
    bench_refill::<BTreeSetMaster>(1000000, 100);
    bench_refill::<BTreeSetMaster>(10000000, 8);


    bench_refill::<SplayMaster>(100, 260000);
    bench_refill::<SplayMaster>(1000, 28000);
    bench_refill::<SplayMaster>(10000, 3000);
    bench_refill::<SplayMaster>(100000, 220);
    bench_refill::<SplayMaster>(1000000, 25);
    bench_refill::<SplayMaster>(10000000, 3);


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

    use treap::TreapMap;
    use teardown_tree::{TeardownTreeRefill};
    use teardown_tree::TeardownTreeSet;
    use teardown_tree::util::make_teardown_seq;
    use super::nanos;
    use cpuprofiler::PROFILER;

    pub type Tree = TeardownTreeSet<usize>;
    pub type TreeBulk = TeardownTreeBulk;


    pub fn btree_single_delete_n(n: usize, rm_items: usize, iters: u64) {
        let mut rng = XorShiftRng::from_seed([1,2,3,4]);
        let mut elapsed_nanos = 0;
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

            let start = time::SystemTime::now();
            for i in 0..rm_items {
                let x = btset.remove(&keys[i]);
                black_box(x);
            }
            let elapsed = start.elapsed().unwrap();
            elapsed_nanos += nanos(elapsed);
        }

        println!("average time to delete {} random elements from BTreeMap using remove(), {} elements: {}ns", rm_items, n, elapsed_nanos/iters)
    }

    pub fn imptree_single_elem_range_n(n: usize, rm_items: usize, iters: u64) {
        let mut rng = XorShiftRng::from_seed([1,2,3,4]);
        let mut elapsed_nanos = 0;

        let elems: Vec<_> = (1..n+1).collect();

        let tree = TeardownTreeBulk(Tree::new(elems));
        let mut copy = tree.clone();
        let mut output = Vec::with_capacity(tree.0.size());

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


            let start = time::SystemTime::now();
            for i in 0..rm_items {
                output.truncate(0);
                let x = copy.0.delete_range(keys[i]..keys[i]+1, &mut output);
                black_box(x);
            }
            let elapsed = start.elapsed().unwrap();
            elapsed_nanos += nanos(elapsed);
        }

        println!("average time to delete {} random elements from TeardownTree using delete_range(), {} elements: {}ns, total: {}ms", rm_items, n, elapsed_nanos/iters, elapsed_nanos/1000000)
    }


    pub fn bench_refill<M: TeardownTreeMaster>(n: usize, iters: u64) {
        let elems: Vec<_> = (0..n).collect();
        let tree = build::<M>(elems);
        let mut copy = tree.cpy();
        let mut elapsed_nanos = 0;

        for _ in 0..iters {
            copy = black_box(copy);
            copy.clear();
            let start = time::SystemTime::now();
            copy.rfill(&tree);
            let elapsed = start.elapsed().unwrap();
            elapsed_nanos += nanos(elapsed);
        }

        println!("average time to refill {} with {} elements: {}ns, total: {}ms", M::descr_refill(), n, elapsed_nanos/iters, elapsed_nanos/1000000)
    }

    #[inline(never)]
    pub fn bench_refill_teardown_cycle<M: TeardownTreeMaster>(n: usize, rm_items: usize, iters: u64) {
        let mut rng = XorShiftRng::from_seed([1,2,3,4]);
        let elems: Vec<_> = (0..n).collect();

        let ranges = make_teardown_seq(n, rm_items, &mut rng);

        let tree = build::<M>(elems);
        let mut copy = tree.cpy();
        let mut output = Vec::with_capacity(tree.sz());
        copy.del_range(0..n, &mut output);
        output.truncate(0);

        PROFILER.lock().unwrap().start("./my-prof.profile").expect("Couldn't start");
        let start = time::SystemTime::now();
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
        let elapsed = start.elapsed().unwrap();
        let elapsed_nanos = nanos(elapsed);
        PROFILER.lock().unwrap().stop().unwrap();
        println!("average time to refill/tear down {}, {} elements in bulks of {} elements: {}ns, total: {}ms", M::descr_cycle(), n, rm_items, elapsed_nanos/iters, elapsed_nanos/1000000)
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
        fn rfill(&mut self, master: &Self::Master);
        fn sz(&self) -> usize;
        fn clear(&mut self);
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
            "TeardownTree using delete_range()".to_string()
        }

        fn descr_refill() -> String {
            "TeardownTree".to_string()
        }
    }

    impl TeardownTreeCopy for TeardownTreeBulk {
        type Master = TeardownTreeBulk;
        type T = usize;

        fn del_range(&mut self, range: Range<usize>, output: &mut Vec<usize>) {
            self.0.delete_range(range, output);
        }

        fn rfill(&mut self, master: &Self::Master) {
            self.0.refill(&master.0)
        }

        fn sz(&self) -> usize {
            self.0.size()
        }

        fn clear(&mut self) {
            self.0.clear();
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
            "TeardownTree using delete()".to_string()
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

        fn rfill(&mut self, master: &Self::Master) {
            self.0.refill(&master.0)
        }

        fn sz(&self) -> usize {
            self.0.size()
        }

        fn clear(&mut self) {
            self.0.clear();
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
            "BTreeSet using remove()".to_string()
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
            "Treap using split/join".to_string()
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

        fn rfill(&mut self, master: &Self::Master) {
            self.0 = master.0.clone()
        }

        fn sz(&self) -> usize {
            self.0.len()
        }

        fn clear(&mut self) {
            self.0.clear();
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
    use splay::SplaySet;

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
            "SplayTree using remove_range()".to_string()
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

        fn rfill(&mut self, master: &Self::Master) {
            self.0 = master.0.clone()
        }

        fn sz(&self) -> usize {
            self.0.len()
        }

        fn clear(&mut self) {
            self.0.clear();
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


    use teardown_tree::{IntervalTeardownTreeSet, KeyInterval};

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
            "IntervalTeardownTree using delete_range()".to_string()
        }

        fn descr_refill() -> String {
            "IntervalTeardownTree".to_string()
        }
    }

    impl TeardownTreeCopy for IntervalTreeBulk {
        type Master = IntervalTreeBulk;
        type T = KeyInterval<usize>;

        fn del_range(&mut self, range: Range<usize>, output: &mut Vec<Self::T>) {
            self.0.delete_intersecting(&KeyInterval::new(range.start, range.end), output);
        }

        fn rfill(&mut self, master: &Self::Master) {
            self.0.refill(&master.0)
        }

        fn sz(&self) -> usize {
            self.0.size()
        }

        fn clear(&mut self) {
            self.0.clear();
        }
    }


    impl Display for IntervalTreeBulk {
        fn fmt(&self, fmt: &mut Formatter) -> Result {
            Display::fmt(&self.0, fmt)
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
}
