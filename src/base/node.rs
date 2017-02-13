use std::ops::{Deref, DerefMut};
use std::cmp::Ordering;

use base::Key;

#[derive(Debug, Clone, Copy)]
pub struct Entry<K, V> {
    item: (K, V)
}

impl<K, V> Entry<K, V> {
    pub fn new(key: K, val: V) -> Self {
        Entry { item: (key, val) }
    }

    #[inline(always)]
    pub fn into_tuple(self) -> (K, V) {
        self.into()
    }

    #[inline(always)] pub fn key(&self) -> &K { &self.item.0 }
    #[inline(always)] pub fn key_mut(&mut self) -> &mut K { &mut self.item.0 }

    #[inline(always)] pub fn val(&self) -> &V { &self.item.1 }
    #[inline(always)] pub fn val_mut(&mut self) -> &mut V { &mut self.item.1 }
}


impl<K: Ord+Clone, V> PartialEq for Entry<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}
impl<K: Ord+Clone, V> Eq for Entry<K, V> {}

impl<K: Ord+Clone, V> PartialOrd for Entry<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.key().cmp(other.key()))
    }
}

impl<K: Ord+Clone, V> Ord for Entry<K, V> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(other.key())
    }
}





impl<K, V> Into<(K,V)> for Entry<K, V> {
    #[inline(always)] fn into(self) -> (K,V) {
        self.item
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
    #[inline] fn into_tuple(self) -> (Self::K, Self::V);
}
