use base::Item;
//use std::ptr::Unique;
use std::{mem, ptr};
use std::fmt::{Debug, Formatter};


pub struct UnsafeStack<T: Sized> {
    pub size: usize,
    pub data: *mut T,
    pub capacity: usize
}

impl<T: Sized+Clone> UnsafeStack<T> {
    pub fn new(capacity: usize) -> UnsafeStack<T> {
        unsafe {
            let mut data = vec![mem::uninitialized(); capacity];
            let ptr: *mut T = data.as_mut_ptr();
            mem::forget(data);
            UnsafeStack { size: 0, data: ptr, capacity: capacity }
        }
    }

    #[inline(always)]
    pub fn push(&mut self, item: T) {
        debug_assert!(self.size < self.capacity);
        unsafe {
            ptr::write(self.data.offset(self.size as isize), item);
        }
        self.size += 1;
    }

    #[inline(always)]
    pub fn pop(&mut self) -> T {
        debug_assert!(self.size > 0);
        self.size -= 1;
        unsafe {
            ptr::read(self.data.offset(self.size as isize))
        }
    }


    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.size
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl <T: Item> Debug for UnsafeStack<T> {
    fn fmt(&self, fmt: &mut Formatter) -> ::std::fmt::Result {
        let result = write!(fmt, "UnsafeStack: {{size={}}}", self.size);
        result
    }
}

impl<T> Drop for UnsafeStack<T> {
    fn drop(&mut self) {
        unsafe {
            Vec::from_raw_parts(self.data, self.size, self.capacity);
            // let it drop
        }
    }
}
