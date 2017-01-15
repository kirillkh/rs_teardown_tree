use std::ops::Range;
use rand::{Rng, XorShiftRng};

#[inline(never)]
pub fn make_teardown_seq(n: usize, rm_items: usize, rng: &mut XorShiftRng) -> Vec<Range<usize>> {
    let nranges = n / rm_items +
        if n % rm_items != 0 { 1 } else { 0 };

    // generate a random permutation
    let mut pool: Vec<_> = (0..nranges).collect();
    let mut ranges = vec![];

    for i in 0..nranges {
        let k = rng.gen_range(0, nranges - i);
        let range_idx = pool.swap_remove(k);
        let from = range_idx * rm_items;
        let to = ::std::cmp::min(from + rm_items, n);
        ranges.push(from..to);
    }

    ranges
}
