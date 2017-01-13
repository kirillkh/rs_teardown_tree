use std::ops::{Deref, DerefMut};

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
    fn into(self) -> (K,V) {
        (self.key, self.val)
    }
}


pub trait Node<K, V>: Deref<Target=KeyVal<K,V>> + DerefMut<Target=KeyVal<K,V>> { //+ Into<KeyVal<K, V>> {
    fn new(key: K, val: V) -> Self;
    fn into_kv(self) -> KeyVal<K,V>;
}
