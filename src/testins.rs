#![cfg_attr(feature = "unstable", feature(test))]


extern crate rand;
#[macro_use] extern crate derive_new;

mod base;
mod applied;
mod external_api;

mod rust_bench;

pub use self::external_api::{IntervalTeardownMap, IntervalTeardownSet, Interval, KeyInterval,
                             TeardownMap, TeardownSet, Refill,
                             iter};
pub use self::base::{ItemFilter, NoopFilter, Sink};
pub use self::base::sink;
pub use self::base::util;



use self::applied::plain_tree::tests::test_insert;


use std::env;

pub fn main() {
    test_insert(env::args().nth(1).unwrap().parse::<usize>().unwrap());
}
