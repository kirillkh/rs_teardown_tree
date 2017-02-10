use std::ops::{Deref, DerefMut};
use std::cmp::Ordering;

use base::Key;

#[derive(Debug, Clone, Copy)]
pub struct Entry<K, V> {
    pub key: K,
    pub val: V
}

impl<K, V> Entry<K, V> {
    pub fn new(key: K, val: V) -> Self {
        Entry { key: key, val: val }
    }
}


impl<K: Ord+Clone, V> PartialEq for Entry<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl<K: Ord+Clone, V> Eq for Entry<K, V> {}

impl<K: Ord+Clone, V> PartialOrd for Entry<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.key.cmp(&other.key))
    }
}

impl<K: Ord+Clone, V> Ord for Entry<K, V> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}





impl<K, V> Into<(K,V)> for Entry<K, V> {
    #[inline] fn into(self) -> (K,V) {
        (self.key, self.val)
    }
}


pub trait Node: Deref<Target= Entry<<Self as Node>::K,
                                    <Self as Node>::V>> +
                DerefMut
{
    type K: Key;
    type V;

    #[inline] fn new(key: Self::K, val: Self::V) -> Self;
    #[inline] fn into_entry(self) -> Entry<Self::K, Self::V>;
}
