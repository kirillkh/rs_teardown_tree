//use std::ptr::Unique;
use std::mem;
use std::fmt::{Debug, Formatter};


#[derive(Clone, Copy)]
pub struct Slot {
    pub idx: usize,
}

impl Debug for Slot {
    fn fmt(&self, fmt: &mut Formatter) -> ::std::fmt::Result {
        write!(fmt, "{}", self.idx)
    }
}


pub struct SlotStack {
    pub nslots: usize,
    pub nfilled: usize,
//    pub slots: Unique<T>, // uncomment when Unique is stabilized
    pub slots: *mut Slot,
    pub capacity: usize
}

impl SlotStack {
    pub fn new(capacity: usize) -> SlotStack {
        unsafe {
            let mut slots = vec![mem::uninitialized(); capacity];
            let ptr: *mut Slot = slots.as_mut_ptr();
            mem::forget(slots);
            SlotStack { nslots: 0, nfilled: 0, slots: ptr, capacity: capacity }
        }
    }

    #[inline(always)]
    pub fn push(&mut self, idx: usize) {
        debug_assert!(self.nslots < self.capacity);
        self.slot_at(self.nslots).idx = idx;
        self.nslots += 1;
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Slot {
        debug_assert!(self.nslots > 0);
        if self.nfilled == self.nslots {
            self.nfilled -= 1;
        }
        self.nslots -= 1;

        *self.slot_at(self.nslots)
    }

    #[inline(always)]
    pub fn peek(&mut self) -> Slot {
        debug_assert!(self.nslots > 0);
        *self.slot_at(self.nslots - 1)
    }


    #[inline(always)]
    pub fn fill(&mut self) -> usize {
        debug_assert!(self.nfilled < self.nslots);
        let dst_idx = self.slot_at(self.nfilled).idx;
        self.nfilled += 1;
        dst_idx
    }


    #[inline(always)]
    pub fn slot_at(&self, idx: usize) -> &mut Slot {
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

//    #[inline(always)]
//    pub fn nfilled(&self) -> usize {
//        self.nfilled
//    }

    #[inline(always)]
    pub fn has_open(&self) -> bool {
        self.nslots != self.nfilled
    }
}

impl Debug for SlotStack {
    fn fmt(&self, fmt: &mut Formatter) -> ::std::fmt::Result {
        let slots = unsafe { Vec::from_raw_parts(self.slots, self.nslots, self.capacity) };
        let result = write!(fmt, "SlotStack: {{nslots={}, nfilled={}, slots={:?}}}", self.nslots, self.nfilled, &slots);
        mem::forget(slots);
        result
    }
}

// Uncomment when Specialization is stabilized.
//impl <T: Item+Debug> Debug for SlotStack<T> {
//    fn fmt(&self, fmt: &mut Formatter) -> ::std::fmt::Result {
//        unsafe {
//            let ptr: *mut Slot<T> = mem::transmute(self.slots.get());
//            let slots_vec = Vec::from_raw_parts(ptr, self.nfilled, self.capacity);
//            let result = write!(fmt, "SlotStack: nslots={}, nfilled={}, slots={:?}", self.nslots, self.nfilled, &slots_vec);
//            mem::forget(slots_vec);
//            result
//        }
//    }
//}

impl Drop for SlotStack {
    fn drop(&mut self) {
        unsafe {
            Vec::from_raw_parts(self.slots, self.nfilled, self.capacity);
            // let it drop
        }
    }
}
