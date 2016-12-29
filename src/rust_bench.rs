#[cfg(all(feature = "unstable", test))]
mod benches {
    extern crate test;

    use self::test::Bencher;

    use base::{TreeWrapper, TreeBase, TeardownTreeRefill};
    use applied::plain_tree::PlainDeleteInternal;

    type Tree = TreeWrapper<usize>;

    #[bench]
    fn bench_delete_range_00100(bencher: &mut Bencher) {
        bench_delete_range_n(100, bencher);
    }

    #[bench]
    fn bench_delete_range_01022(bencher: &mut Bencher) {
        bench_delete_range_n(1022, bencher);
    }

    #[bench]
    fn bench_delete_range_01023(bencher: &mut Bencher) {
        bench_delete_range_n(1023, bencher);
    }

    #[bench]
    fn bench_delete_range_02046(bencher: &mut Bencher) {
        bench_delete_range_n(2046, bencher);
    }

    #[bench]
    fn bench_delete_range_02047(bencher: &mut Bencher) {
        bench_delete_range_n(2047, bencher);
    }

    #[bench]
    fn bench_delete_range_04094(bencher: &mut Bencher) {
        bench_delete_range_n(4094, bencher);
    }

    #[bench]
    fn bench_delete_range_04095(bencher: &mut Bencher) {
        bench_delete_range_n(4095, bencher);
    }

    #[bench]
    fn bench_delete_range_05000(bencher: &mut Bencher) {
        bench_delete_range_n(5000, bencher);
    }

    #[bench]
    fn bench_delete_range_08190(bencher: &mut Bencher) {
        bench_delete_range_n(8190, bencher);
    }

    #[bench]
    fn bench_delete_range_08191(bencher: &mut Bencher) {
        bench_delete_range_n(8191, bencher);
    }

    #[bench]
    fn bench_delete_range_10000(bencher: &mut Bencher) {
        bench_delete_range_n(10000, bencher);
    }

    #[bench]
    fn bench_delete_range_16000(bencher: &mut Bencher) {
        bench_delete_range_n(16000, bencher);
    }

    #[bench]
    fn bench_delete_range_16381(bencher: &mut Bencher) {
        bench_delete_range_n(16381, bencher);
    }

    #[bench]
    fn bench_delete_range_16382(bencher: &mut Bencher) {
        bench_delete_range_n(test::black_box(16382), bencher);
    }

    #[bench]
    fn bench_delete_range_16383(bencher: &mut Bencher) {
        bench_delete_range_n(test::black_box(16383), bencher);
    }

    #[bench]
    fn bench_delete_range_25000(bencher: &mut Bencher) {
        bench_delete_range_n(25000, bencher);
    }

    #[bench]
    fn bench_delete_range_50000(bencher: &mut Bencher) {
        bench_delete_range_n(50000, bencher);
    }

    //    #[bench]
    //    fn bench_delete_range_100000(bencher: &mut Bencher) {
    //        bench_delete_range_n(100000, bencher);
    //    }
    //
    //    #[bench]
    //    fn bench_delete_range_10000000(bencher: &mut Bencher) {
    //        bench_delete_range_n(10000000, bencher);
    //    }

    #[inline(never)]
    fn bench_delete_range_n(n: usize, bencher: &mut Bencher) {
        let elems: Vec<_> = (1..n + 1).collect();

        let perm = {
            // generate a random permutation
            let mut pool: Vec<_> = (1..101).collect();
            let mut perm = vec![];

            use rand::{XorShiftRng, SeedableRng, Rng};

            let mut rng = XorShiftRng::from_seed([1, 2, 3, 4]);

            for i in 0..100 {
                let n: u32 = rng.gen_range(0, 100 - i);
                let next = pool.swap_remove(n as usize);
                perm.push(next);
            }

            perm
        };


        let tree = Tree::new(elems);
        let mut copy = tree.clone();
        let mut output = Vec::with_capacity(tree.size());

        bencher.iter(|| {
            copy.refill(&tree);
            for i in 0..100 {
                output.truncate(0);
                let x = perm[i];
                copy.delete_range((x - 1) * n / 100, x * n / 100, &mut output);
                test::black_box(output.len());
            }
        });
    }
}
