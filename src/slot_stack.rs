use base::Item;
use std::ptr::Unique;
use std::{mem, ptr};
use std::fmt::{Debug, Formatter};


type Slot<T> = Option<T>;

pub struct SlotStack<T: Item> {
    pub nslots: usize,
    pub nfilled: usize,
    pub slots: Unique<T>,
    pub capacity: usize
}

impl<T: Item> SlotStack<T> {
    pub fn new(capacity: usize) -> SlotStack<T> {
        unsafe {
            let mut slots = vec![mem::uninitialized(); capacity];
            let ptr: *mut T = slots.as_mut_ptr();
            mem::forget(slots);
            SlotStack { nslots: 0, nfilled: 0, slots: Unique::new(ptr), capacity: capacity }
        }
    }

    #[inline(always)]
    pub fn push(&mut self) {
        debug_assert!(self.nslots < self.capacity);
        self.nslots += 1;
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Slot<T> {
        debug_assert!(self.nslots > 0);
        if self.nfilled == self.nslots {
            self.nfilled -= 1;
            self.nslots -= 1;
            unsafe {
                Some(ptr::read(self.slot_at(self.nslots) as *const T))
            }
        } else {
            self.nslots -= 1;
            None
        }
    }

    #[inline(always)]
    pub fn fill(&mut self, item: T) {
        debug_assert!(self.nfilled < self.nslots);
        *self.slot_at(self.nfilled) = item;
        self.nfilled += 1;
    }

    #[inline(always)]
    pub fn fill_opt(&mut self, item: Option<T>) {
        debug_assert!(item.is_some());
        debug_assert!(self.nfilled < self.nslots);
        *self.slot_at(self.nfilled) = item.unwrap();
        self.nfilled += 1;
    }


    #[inline(always)]
    fn slot_at(&self, idx: usize) -> &mut T {
        unsafe {
            mem::transmute(self.slots.offset(idx as isize))
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.nslots == 0
    }

    #[inline(always)]
    pub fn nslots(&self) -> usize {
        self.nslots
    }

    #[inline(always)]
    pub fn nfilled(&self) -> usize {
        self.nfilled
    }

    #[inline(always)]
    pub fn has_open(&self) -> bool {
        self.nslots != self.nfilled
    }
}

impl <T: Item> Debug for SlotStack<T> {
    default fn fmt(&self, fmt: &mut Formatter) -> ::std::fmt::Result {
        let result = write!(fmt, "SlotStack: {{nslots={}, nfilled={}}}", self.nslots, self.nfilled);
        result
    }
}

impl <T: Item+Debug> Debug for SlotStack<T> {
    fn fmt(&self, fmt: &mut Formatter) -> ::std::fmt::Result {
        unsafe {
            let ptr: *mut Slot<T> = mem::transmute(self.slots.get());
            let slots_vec = Vec::from_raw_parts(ptr, self.nfilled, self.capacity);
            let result = write!(fmt, "SlotStack: nslots={}, nfilled={}, slots={:?}", self.nslots, self.nfilled, &slots_vec);
            mem::forget(slots_vec);
            result
        }
    }
}

impl <T: Item> Drop for SlotStack<T> {
    fn drop(&mut self) {
        unsafe {
            Vec::from_raw_parts(self.slots.get_mut(), self.nfilled, self.capacity);
            // let it drop
        }
    }
}
