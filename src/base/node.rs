use std::ops::{Deref, DerefMut};

use base::Key;

#[derive(Debug, Clone)]
pub struct KeyVal<K, V> {
    pub key: K,
    pub val: V
}

impl<K, V> KeyVal<K, V> {
    pub fn new(key: K, val: V) -> Self {
        KeyVal { key: key, val: val }
    }
}

impl<K, V> Into<(K,V)> for KeyVal<K, V> {
    #[inline] fn into(self) -> (K,V) {
        (self.key, self.val)
    }
}


pub trait Node: Deref<Target=KeyVal<<Self as Node>::K,
                                    <Self as Node>::V>> +
                DerefMut
{
    type K: Key;
    type V;

    #[inline] fn new(key: Self::K, val: Self::V) -> Self;
    #[inline] fn into_kv(self) -> KeyVal<Self::K, Self::V>;
}
