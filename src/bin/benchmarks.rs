#![feature(test)]
extern crate test;
extern crate rand;
extern crate implicit_tree;
use std::time;

use std::collections::BTreeMap;
use std::time::Duration;
use rand::{XorShiftRng, SeedableRng, Rng};

use implicit_tree::{ImplicitTree, ImplicitTreeRefill, DriverFromTo};

type Tree = ImplicitTree<usize>;

fn btree_single_delete_n(n: usize, rm_items: usize, iters: u64) {
    let mut rng = XorShiftRng::from_seed([1,2,3,4]);
    let mut elapsed_nanos = 0;
    for _ in 0..iters {
        let mut btmap = BTreeMap::new();
        for i in 0..n {
            btmap.insert(i, i);
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
            let x = btmap.remove(&keys[i]);
            test::black_box(x);
        }
        let elapsed = start.elapsed().unwrap();
        elapsed_nanos += nanos(elapsed);
    }

    println!("average time to delete {} elements from BTreeMap of {} elements: {}ns", rm_items, n, elapsed_nanos/iters)
}

fn imptree_single_delete_n(n: usize, rm_items: usize, iters: u64) {
    let mut rng = XorShiftRng::from_seed([1,2,3,4]);
    let mut elapsed_nanos = 0;

    let elems: Vec<_> = (1..n+1).collect();

    let tree = Tree::new(elems);
    let mut copy = tree.clone();
    let mut output = Vec::with_capacity(tree.size());

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

        copy.refill(&tree);


        let start = time::SystemTime::now();
        for i in 0..rm_items {
            output.truncate(0);
            let x = copy.delete_bulk(&mut DriverFromTo::new(keys[i], keys[i]), &mut output);
            test::black_box(x);
        }
        let elapsed = start.elapsed().unwrap();
        elapsed_nanos += nanos(elapsed);
    }

    println!("average time to delete {} elements from implicit_tree of {} elements: {}ns", rm_items, n, elapsed_nanos/iters)
}



fn btree_bulk_delete_n(n: usize, rm_items: usize, iters: u64) {
    let mut rng = XorShiftRng::from_seed([1,2,3,4]);
    let mut elapsed_nanos = 0;
    for _ in 0..iters {
        let mut btmap = BTreeMap::new();
        for i in 0..n {
            btmap.insert(i, i);
        }

        let from =
            if n > rm_items { rng.gen_range(0, n - rm_items) }
            else { 0 };
        let keys: Vec<_> =  (from..n).collect();

        let start = time::SystemTime::now();
        for i in 0..rm_items {
            let x = btmap.remove(&keys[i]);
            test::black_box(x);
        }
        let elapsed = start.elapsed().unwrap();
        elapsed_nanos += nanos(elapsed);
    }

    println!("average time to bulk delete {} elements from BTreeMap of {} elements: {}ns", rm_items, n, elapsed_nanos/iters)
}

fn imptree_bulk_delete_n(n: usize, rm_items: usize, iters: u64) {
    let mut rng = XorShiftRng::from_seed([1,2,3,4]);
    let mut elapsed_nanos = 0;

    let elems: Vec<_> = (1..n+1).collect();
    let tree = Tree::new(elems);
    let mut copy = tree.clone();
    let mut output = Vec::with_capacity(tree.size());


    for _ in 0..iters {
        let from =
            if n > rm_items { rng.gen_range(0, n - rm_items) }
            else { 0 };
        output.truncate(0);
        copy.refill(&tree);

        let start = time::SystemTime::now();
        let x = copy.delete_bulk(&mut DriverFromTo::new(from, from+rm_items), &mut output);
        test::black_box(x);
        let elapsed = start.elapsed().unwrap();
        elapsed_nanos += nanos(elapsed);
    }

    println!("average time to bulk delete {} elements from implicit_tree of {} elements: {}ns", rm_items, n, elapsed_nanos/iters)
}

#[inline]
fn nanos(d: Duration) -> u64 {
    d.as_secs()*1000000000 + d.subsec_nanos() as u64
}


fn main() {
    imptree_bulk_delete_n(100, 100, 10000000);


    imptree_bulk_delete_n(100, 100, 5000000);
    imptree_bulk_delete_n(1000, 100, 1200000);
    imptree_bulk_delete_n(10000, 100, 500000);
    imptree_bulk_delete_n(100000, 100, 30000);

    btree_bulk_delete_n(100, 100, 200000);
    btree_bulk_delete_n(1000, 100, 200000);
    btree_bulk_delete_n(10000, 100, 20000);
    btree_bulk_delete_n(100000, 100, 5000);

    imptree_single_delete_n(100, 100, 100000);
    imptree_single_delete_n(1000, 100, 30000);
    imptree_single_delete_n(10000, 100, 10000);
    imptree_single_delete_n(100000, 100, 800);

    btree_single_delete_n(100, 100, 100000);
    btree_single_delete_n(1000, 100, 30000);
    btree_single_delete_n(10000, 100, 10000);
    btree_single_delete_n(100000, 100, 800);
}
