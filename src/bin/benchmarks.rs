extern crate rand;
extern crate teardown_tree___treap;
extern crate teardown_tree;
//extern crate wio;
use std::time;

use teardown_tree___treap::TreapMap;

use std::collections::BTreeSet;
use std::time::Duration;
use rand::{XorShiftRng, SeedableRng, Rng};

use teardown_tree::{TeardownTree, TeardownTreeRefill};

type Tree = TeardownTree<usize>;
type TreeBulk = TeardownTreeBulk;



fn btree_single_delete_n(n: usize, rm_items: usize, iters: u64) {
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

fn imptree_single_elem_range_n(n: usize, rm_items: usize, iters: u64) {
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
            let x = copy.0.delete_range(keys[i], keys[i], &mut output);
            black_box(x);
        }
        let elapsed = start.elapsed().unwrap();
        elapsed_nanos += nanos(elapsed);
    }

    println!("average time to delete {} random elements from TeardownTree using delete_range(), {} elements: {}ns, total: {}ms", rm_items, n, elapsed_nanos/iters, elapsed_nanos/1000000)
}


fn bench_refill<M: TeardownTreeMaster>(n: usize, iters: u64) {
    let elems: Vec<_> = (0..n).collect();
    let tree = M::build(elems);
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
fn bench_refill_teardown_cycle<M: TeardownTreeMaster>(n: usize, rm_items: usize, iters: u64) {
    let mut rng = XorShiftRng::from_seed([1,2,3,4]);
    let elems: Vec<_> = (0..n).collect();

    let nranges = n / rm_items +
        if n % rm_items != 0 { 1 } else { 0 };

    let ranges = {
        // generate a random permutation
        let mut pool: Vec<_> = (0..nranges).collect();
        let mut ranges = vec![];

        for i in 0..nranges {
            let k = rng.gen_range(0, nranges-i);
            let range_idx = pool.swap_remove(k);
            let from = range_idx * rm_items;
            let to = ::std::cmp::min(from + rm_items-1, n-1);
            ranges.push((from, to));
        }

        ranges
    };


    let tree = M::build(elems);
    let mut copy = tree.cpy();
    let mut output = Vec::with_capacity(tree.sz());
    copy.del_range(0, n-1, &mut output);
    output.truncate(0);

    let start = time::SystemTime::now();
    for _ in 0..iters {
        copy.rfill(&tree);
        for i in 0..nranges {
            output.truncate(0);
            let (ref from, ref to) = ranges[i];
            copy.del_range(*from, *to, &mut output);
            output = black_box(output);
            assert!(output.len() == *to  - *from + 1, "from={}, to={}, expected: {}, len: {}", *from, *to, *to  - *from + 1, output.len());
        }
        assert!(copy.sz() == 0);
    }
    let elapsed = start.elapsed().unwrap();
    let elapsed_nanos = nanos(elapsed);
    println!("average time to refill/tear down {}, {} elements in bulks of {} elements: {}ns, total: {}ms", M::descr_cycle(), n, rm_items, elapsed_nanos/iters, elapsed_nanos/1000000)
}




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
    bench_refill::<TreeBulk>(100, 40000000);
    bench_refill::<TreeBulk>(1000, 6000000);
    bench_refill::<TreeBulk>(10000, 500000);
    bench_refill::<TreeBulk>(100000, 40000);
    bench_refill::<TreeBulk>(1000000, 1200);
    bench_refill::<TreeBulk>(10000000, 110);


    bench_refill::<TreapMaster>(100, 260000);
    bench_refill::<TreapMaster>(1000, 28000);
    bench_refill::<TreapMaster>(10000, 3000);
    bench_refill::<TreapMaster>(100000, 220);
    bench_refill::<TreapMaster>(1000000, 25);
    bench_refill::<TreapMaster>(10000000, 3);


    bench_refill::<BTreeSet<usize>>(100, 1700000);
    bench_refill::<BTreeSet<usize>>(1000, 180000);
    bench_refill::<BTreeSet<usize>>(10000, 16000);
    bench_refill::<BTreeSet<usize>>(100000, 1300);
    bench_refill::<BTreeSet<usize>>(1000000, 100);
    bench_refill::<BTreeSet<usize>>(10000000, 8);



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


    bench_refill_teardown_cycle::<BTreeSet<usize>>(100, 100, 500000);
    bench_refill_teardown_cycle::<BTreeSet<usize>>(1000, 100, 50000);
    bench_refill_teardown_cycle::<BTreeSet<usize>>(10000, 100, 4000);
    bench_refill_teardown_cycle::<BTreeSet<usize>>(100000, 100, 350);
    bench_refill_teardown_cycle::<BTreeSet<usize>>(1000000, 100, 30);
    bench_refill_teardown_cycle::<BTreeSet<usize>>(10000000, 100, 4);

    bench_refill_teardown_cycle::<BTreeSet<usize>>(1000, 1000, 50000);
    bench_refill_teardown_cycle::<BTreeSet<usize>>(10000, 1000, 6000);
    bench_refill_teardown_cycle::<BTreeSet<usize>>(100000, 1000, 600);
    bench_refill_teardown_cycle::<BTreeSet<usize>>(1000000, 1000, 40);
    bench_refill_teardown_cycle::<BTreeSet<usize>>(10000000, 1000, 4);


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

trait TeardownTreeMaster: Sized {
    type Cpy: TeardownTreeCopy<Master=Self>;

    fn build(elems: Vec<usize>) -> Self;
    fn cpy(&self) -> Self::Cpy;
    fn sz(&self) -> usize;
    fn descr_cycle() -> String;
    fn descr_refill() -> String;
}

trait TeardownTreeCopy {
    type Master: TeardownTreeMaster;

    fn del_range(&mut self, from: usize, to: usize, output: &mut Vec<usize>);
    fn rfill(&mut self, master: &Self::Master);
    fn sz(&self) -> usize;
    fn clear(&mut self);
}


/// for benchmarking TeardownTree delete_range()
#[derive(Clone, Debug)]
struct TeardownTreeBulk(TeardownTree<usize>);

impl TeardownTreeMaster for TeardownTreeBulk {
    type Cpy = TeardownTreeBulk;

    fn build(elems: Vec<usize>) -> Self {
        TeardownTreeBulk(TeardownTree::new(elems))
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

    fn del_range(&mut self, from: usize, to: usize, output: &mut Vec<usize>) {
        self.0.delete_range(from, to, output);
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



/// for benchmarking TeardownTree delete()
#[derive(Clone, Debug)]
struct TeardownTreeSingle(TeardownTree<usize>);

impl TeardownTreeMaster for TeardownTreeSingle {
    type Cpy = TeardownTreeSingle;

    fn build(elems: Vec<usize>) -> Self {
        TeardownTreeSingle(TeardownTree::new(elems))
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

    fn del_range(&mut self, from: usize, to: usize, output: &mut Vec<usize>) {
        for i in from..to+1 {
            if let Some(x) = self.0.delete(&i) {
                output.push(x);
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



/// for benchmarking BTreeSet remove()
impl TeardownTreeMaster for BTreeSet<usize> {
    type Cpy = BTreeSetCopy;

    fn build(elems: Vec<usize>) -> Self {
        let mut set = BTreeSet::new();

        for elem in elems.into_iter() {
            set.insert(elem);
        }

        set
    }

    fn cpy(&self) -> Self::Cpy {
        BTreeSetCopy { set: self.clone() }
    }

    fn sz(&self) -> usize {
        self.len()
    }

    fn descr_cycle() -> String {
        "BTreeSet using remove()".to_string()
    }

    fn descr_refill() -> String {
        "BTreeSet".to_string()
    }
}

struct BTreeSetCopy {
    set: BTreeSet<usize>
}

impl TeardownTreeCopy for BTreeSetCopy {
    type Master = BTreeSet<usize>;

    fn del_range(&mut self, from: usize, to: usize, output: &mut Vec<usize>) {
        for i in from..to+1 {
            if self.set.remove(&i) {
                output.push(i);
            }
        }
    }

    fn rfill(&mut self, master: &Self::Master) {
        assert!(self.set.is_empty(), "size={}", self.set.len());
        self.set = master.clone();
    }

    fn sz(&self) -> usize {
        self.set.len()
    }

    fn clear(&mut self) {
        self.set.clear();
    }
}


fn black_box<T>(dummy: T) -> T {
    use std::ptr;
    use std::mem::forget;

    unsafe {
        let ret = ptr::read_volatile(&dummy as *const T);
        forget(dummy);
        ret
    }
}



//---- benchmarking Treap split/join ---------------------------------------------------------------
use std::iter::FromIterator;

struct TreapMaster (TreapMap<usize, ()>);
struct TreapCopy (TreapMap<usize, ()>);

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

    fn del_range(&mut self, from: usize, to: usize, output: &mut Vec<usize>) {
        self.0.delete_range(from, to+1, output);
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
