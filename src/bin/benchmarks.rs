extern crate rand;
extern crate teardown_tree;
//extern crate wio;
extern crate cpuprofiler;
use std::time;

use std::collections::BTreeSet;
use std::time::Duration;
use rand::{XorShiftRng, SeedableRng, Rng};
use cpuprofiler::PROFILER;

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

    println!("average time to delete {} random elements from BTreeMap of {} elements: {}ns", rm_items, n, elapsed_nanos/iters)
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

    println!("average time to delete {} random elements from TeardownTreeBulk, {} elements: {}ns", rm_items, n, elapsed_nanos/iters)
}


fn bench_delete_range_n<M: TeardownTreeMaster>(n: usize, rm_items: usize, iters: u64) {
    let mut rng = XorShiftRng::from_seed([1,2,3,4]);
    let mut elapsed_nanos = 0;

    let elems: Vec<_> = (0..n).collect();
    let tree = M::build(elems);
    let mut copy = tree.cpy();
    let mut output = Vec::with_capacity(tree.sz());

    for _ in 0..iters {
        let from =
            if n > rm_items { rng.gen_range(0, n - rm_items) }
            else { 0 };
        output.truncate(0);
        copy.del_range(0, n+1, &mut output);
        output = black_box(output);
        output.truncate(0);
        copy.rfill(&tree);

        let start = time::SystemTime::now();
        copy.del_range(from, from+rm_items-1, &mut output);
        output = black_box(output);
        assert!(output.len() == rm_items);
        let elapsed = start.elapsed().unwrap();
        elapsed_nanos += nanos(elapsed);
    }

    println!("average time to delete range of {} elements from {}, {} elements: {}ns", rm_items, M::descr(), n, elapsed_nanos/iters)
}

#[inline(never)]
fn bench_clone_teardown_cycle<M: TeardownTreeMaster>(n: usize, rm_items: usize, iters: u64) {
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

    PROFILER.lock().unwrap().start("./my-prof.profile").expect("Couldn't start");
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
    let avg_nanos = nanos(elapsed) / iters;
    PROFILER.lock().unwrap().stop().unwrap();
    println!("average time to clone/tear down {}, {} elements in bulks of {} elements: {}ns", M::descr(), n, rm_items, avg_nanos)
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
//    imptree_delete_range_n(100, 100, 10000000);

//    // TEST
//    bench_delete_range_n::<Tree>(1000000, 100, 10000);
//    return;

//    set_affinity();

//    bench_delete_range_n::<Tree>(100000, 100, 15000);
//    bench_delete_range_n::<Tree>(1000000, 100, 2000);

    // the most interesting comparisons
//    bench_delete_range_n::<TreeBulk>(1000000, 1000, 3000);
//    bench_delete_range_n::<BTreeSet<usize>>(1000000, 1000, 1000);
//
    bench_clone_teardown_cycle::<TreeBulk>(1000000, 1000, 3000);
//    bench_clone_teardown_cycle::<BTreeSet<usize>>(1000000, 1000, 300);
    return;

    bench_clone_teardown_cycle::<TreeBulk>(100, 100, 3000000);
    bench_clone_teardown_cycle::<TreeBulk>(1000, 100, 300000);
    bench_clone_teardown_cycle::<TreeBulk>(10000, 100, 15000);
    bench_clone_teardown_cycle::<TreeBulk>(100000, 100, 2000);
    bench_clone_teardown_cycle::<TreeBulk>(1000000, 100, 200);

    bench_clone_teardown_cycle::<TreeBulk>(1000, 1000, 1000000);
    bench_clone_teardown_cycle::<TreeBulk>(10000, 1000, 50000);
    bench_clone_teardown_cycle::<TreeBulk>(100000, 1000, 5000);
    bench_clone_teardown_cycle::<TreeBulk>(1000000, 1000, 300);

    bench_delete_range_n::<TreeBulk>(100, 100, 5000000);
    bench_delete_range_n::<TreeBulk>(1000, 100, 800000);
    bench_delete_range_n::<TreeBulk>(10000, 100, 100000);
    bench_delete_range_n::<TreeBulk>(100000, 100, 10000);
    bench_delete_range_n::<TreeBulk>(1000000, 100, 1000);
    bench_delete_range_n::<TreeBulk>(1000000, 1000, 1000);


    bench_clone_teardown_cycle::<BTreeSet<usize>>(100, 100, 50000);
    bench_clone_teardown_cycle::<BTreeSet<usize>>(1000, 100, 15000);
    bench_clone_teardown_cycle::<BTreeSet<usize>>(10000, 100, 8000);
    bench_clone_teardown_cycle::<BTreeSet<usize>>(100000, 100, 2000);
    bench_clone_teardown_cycle::<BTreeSet<usize>>(1000000, 100, 200);

    bench_clone_teardown_cycle::<BTreeSet<usize>>(1000, 1000, 15000);
    bench_clone_teardown_cycle::<BTreeSet<usize>>(10000, 1000, 8000);
    bench_clone_teardown_cycle::<BTreeSet<usize>>(100000, 1000, 2000);
    bench_clone_teardown_cycle::<BTreeSet<usize>>(1000000, 1000, 300);

    bench_delete_range_n::<BTreeSet<usize>>(100, 100, 600000);
    bench_delete_range_n::<BTreeSet<usize>>(1000, 100, 600000);
    bench_delete_range_n::<BTreeSet<usize>>(10000, 100, 20000);
    bench_delete_range_n::<BTreeSet<usize>>(100000, 100, 5000);
    bench_delete_range_n::<BTreeSet<usize>>(1000000, 100, 300);
    bench_delete_range_n::<BTreeSet<usize>>(1000000, 1000, 300);


    bench_clone_teardown_cycle::<TeardownTreeSingle>(100, 100, 50000);
    bench_clone_teardown_cycle::<TeardownTreeSingle>(1000, 100, 15000);
    bench_clone_teardown_cycle::<TeardownTreeSingle>(10000, 100, 8000);
    bench_clone_teardown_cycle::<TeardownTreeSingle>(100000, 100, 2000);
    bench_clone_teardown_cycle::<TeardownTreeSingle>(1000000, 100, 200);

    bench_clone_teardown_cycle::<TeardownTreeSingle>(1000, 1000, 15000);
    bench_clone_teardown_cycle::<TeardownTreeSingle>(10000, 1000, 8000);
    bench_clone_teardown_cycle::<TeardownTreeSingle>(100000, 1000, 2000);
    bench_clone_teardown_cycle::<TeardownTreeSingle>(1000000, 1000, 300);

    bench_delete_range_n::<TeardownTreeSingle>(100, 100, 600000);
    bench_delete_range_n::<TeardownTreeSingle>(1000, 100, 600000);
    bench_delete_range_n::<TeardownTreeSingle>(10000, 100, 20000);
    bench_delete_range_n::<TeardownTreeSingle>(100000, 100, 5000);
    bench_delete_range_n::<TeardownTreeSingle>(1000000, 100, 300);


    imptree_single_elem_range_n(100, 100,    200000);
    imptree_single_elem_range_n(1000, 100,   150000);
    imptree_single_elem_range_n(10000, 100,  100000);
    imptree_single_elem_range_n(100000, 100,  40000);
    imptree_single_elem_range_n(1000000, 100,  6000);

    btree_single_delete_n(100, 100, 100000);
    btree_single_delete_n(1000, 100, 30000);
    btree_single_delete_n(10000, 100, 10000);
    btree_single_delete_n(100000, 100, 800);
    btree_single_delete_n(1000000, 100, 300);


}



//---- unifying interfaces used in above benchmarks and its impls for 1) TeardownTree delete_range, 2) TeardownTree delete(), BTreeSet

trait TeardownTreeMaster: Sized {
    type Cpy: TeardownTreeCopy<Master=Self>;

    fn build(elems: Vec<usize>) -> Self;
    fn cpy(&self) -> Self::Cpy;
    fn sz(&self) -> usize;
    fn descr() -> String;
}

trait TeardownTreeCopy {
    type Master: TeardownTreeMaster;

    fn del_range(&mut self, from: usize, to: usize, output: &mut Vec<usize>);
    fn rfill(&mut self, master: &Self::Master);
    fn sz(&self) -> usize;
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

    fn descr() -> String {
        "TeardownTree using delete_range()".to_string()
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

    fn descr() -> String {
        "TeardownTree using delete()".to_string()
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

    fn descr() -> String {
        "BTreeSet using remove()".to_string()
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
