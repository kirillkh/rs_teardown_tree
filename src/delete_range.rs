use base::{ImplicitTree, Item, Node};
use std::ptr::Unique;

use std::{mem, ptr};

pub trait TraversalDriver<T: Item> {
    #[inline(always)]
    fn decide(&mut self, node: &T) -> TraversalDecision;
}

#[derive(Clone, Copy)]
pub struct TraversalDecision {
    pub traverse_left: bool,
    pub traverse_right: bool,
}

impl TraversalDecision {
    #[inline(always)]
    pub fn consume(&self) -> bool {
        self.traverse_left && self.traverse_right
    }
}

struct RejectDriver;
impl<T: Item> TraversalDriver<T> for RejectDriver {
    fn decide(&mut self, _: &T) -> TraversalDecision {
        TraversalDecision { traverse_left: false, traverse_right: false }
    }
}


type Slot<T> = Option<T>;


struct SlotStack<T: Item> {
    nslots: usize,
    nfilled: usize,
    slots: Unique<T>,
    capacity: usize
}

impl<T: Item> SlotStack<T> {
    fn new(capacity: usize) -> SlotStack<T> {
        unsafe {
            let mut slots = vec![mem::uninitialized(); capacity];
            let ptr: *mut T = slots.as_mut_ptr();
            mem::forget(slots);
            SlotStack { nslots: 0, nfilled: 0, slots: Unique::new(ptr), capacity: capacity }
        }
    }

    #[inline(always)]
    fn push_slot(&mut self) {
        debug_assert!(self.nslots < self.capacity);
        self.nslots += 1;
    }

    #[inline(always)]
    fn pop(&mut self) -> Slot<T> {
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
    fn fill_slot(&mut self, item: T) {
        debug_assert!(self.nfilled < self.nslots);
        *self.slot_at(self.nfilled) = item;
        self.nfilled += 1;
    }

    #[inline(always)]
    fn fill_slot_opt(&mut self, item: Option<T>) {
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

    fn to_str(&self) -> String {
        unsafe {
            let ptr: *mut Slot<T> = mem::transmute(self.slots.get());
            let slots_vec = Vec::from_raw_parts(ptr, self.capacity, self.capacity);
            let str = format!("{:?}", slots_vec);
            mem::forget(slots_vec);
            str
        }
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.nslots == 0
    }

    #[inline(always)]
    fn nslots(&self) -> usize {
        self.nslots
    }

    #[inline(always)]
    fn nfilled(&self) -> usize {
        self.nfilled
    }
}




pub struct DeleteRange<'a, T: 'a+Item> {

    tree: &'a mut ImplicitTree<T>,
    slots_min: SlotStack<T>, slots_max: SlotStack<T>,
    pub output: &'a mut Vec<T>
}

impl<'a, T: Item> DeleteRange<'a, T> {
    pub fn new(tree: &'a mut ImplicitTree<T>, output: &'a mut Vec<T>) -> DeleteRange<'a, T> {
        let height = tree.node(0).height as usize;
        let slots_min = SlotStack::new(height);
        let slots_max = SlotStack::new(height);
        DeleteRange { tree: tree, slots_min: slots_min, slots_max: slots_max, output: output }
    }

    pub fn delete_range<D: TraversalDriver<T>>(&mut self, drv: &mut D) {
//        // TEST
//        let orig = self.tree.clone();

        if !self.tree.is_null(0) {
            self.delete_range_recurse(drv, 0);
            debug_assert!(self.slots_min.is_empty() && self.slots_max.is_empty(),
                    "tree: {:?}, replacements_min: {}, replacements_max: {}, output: {:?}", self.tree, self.slots_min.to_str(), self.slots_max.to_str(), self.output);
        }
    }

    #[inline(never)]
    fn delete_range_recurse<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize) {
        let item: &mut Option<T> = &mut self.node_mut(idx).item;
        let decision = drv.decide(item.as_ref().unwrap());
        let consumed = decision.consume();
        if consumed {
            let item = item.take().unwrap();
            self.output.push(item);
        }

        match (self.tree.has_left(idx), self.tree.has_right(idx)) {
            (false, false) => self.traverse_shape_leaf(idx, consumed),
            (true, false)  => self.traverse_shape_left(drv, idx, decision),
            (false, true)  => self.traverse_shape_right(drv, idx, decision),
            (true, true)   => self.traverse_shape_dual(drv, idx, decision)
        }
    }


    //---- traverse_shape_* ------------------------------------------------------------------------
    #[inline(always)]
    fn traverse_shape_leaf(&mut self, idx: usize, consumed: bool) {
        let node = self.node_mut(idx);
        if consumed {
            node.height = 0;
        } else if self.open_min_slots() {
            let item = node.item.take();
            self.slots_min.fill_slot_opt(item);

            node.height = 0;
        } else if self.open_max_slots() {
            let item = node.item.take();
            self.slots_max.fill_slot_opt(item);

            node.height = 0;
        }
    }

    // tested -- inlining really helps
    #[inline(always)]
    fn traverse_shape_left<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, decision: TraversalDecision) {
        let mut need_replacement = decision.consume();
        let node = self.node_mut(idx);
        if need_replacement {
        } else if self.open_max_slots() {
            let item = node.item.take();
            self.slots_max.fill_slot_opt(item);

            need_replacement = true;
        } else if !self.open_min_slots() && !decision.traverse_left {
            return;
        };

        need_replacement = self.descend_left(drv, idx, need_replacement);

        // fulfill a min req if necessary
        if !need_replacement && self.open_min_slots() {
            let item = node.item.take();
            self.slots_min.fill_slot_opt(item);

            need_replacement = true;
        }

        // update height
        let height = if need_replacement {
            0
        } else {
            let left = self.node_mut(ImplicitTree::<T>::lefti(idx));
            left.height + 1
        };
        node.height = height;
    }

    // tested -- inlining really helps
    #[inline(always)]
    fn traverse_shape_right<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, decision: TraversalDecision) {
        let node = self.node_mut(idx);

        let mut need_replacement = decision.consume();
        if need_replacement {
        } else if self.open_min_slots() {
//            self.move_to_slot(idx, ItemListId::MIN);
            let item = node.item.take();
            self.slots_min.fill_slot_opt(item);

            need_replacement = true;
        } else if !self.open_max_slots() && !decision.traverse_right {
            return;
        }

        need_replacement = self.descend_right(drv, idx, need_replacement);

        // fulfill a max req if necessary
        if !need_replacement && self.open_max_slots() {
//            self.move_to_slot(idx, ItemListId::MAX);
            let item = node.item.take();
            self.slots_max.fill_slot_opt(item);

            need_replacement = true;
        }

        // update height
        let height = if need_replacement {
            0
        } else {
            let right = self.node_mut(ImplicitTree::<T>::righti(idx));
            right.height + 1
        };
        node.height = height;
    }

    // tested -- inlining really helps
    #[inline(always)]
    fn traverse_shape_dual<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize, decision: TraversalDecision) {
        let node = self.node_mut(idx);
        let mut need_replacement = decision.consume();

        // left subtree
        if decision.traverse_left || self.open_min_slots() || need_replacement {
//            let nslots = self.slots_max.nslots();
            let slots_max_left = Self::pin_stack(&mut self.slots_max);
            let slots_max_orig = mem::replace(&mut self.slots_max, slots_max_left);

            {
                need_replacement = self.descend_left(drv, idx, need_replacement);
            }

            let slots_max_left = mem::replace(&mut self.slots_max, slots_max_orig);
            mem::forget(slots_max_left);
        }

        // this node
        if !need_replacement && self.open_min_slots() {
            // this node is the minimum of the tree: use it to fulfill a min request
//            self.move_to_slot(idx, ItemListId::MIN);
            let item = node.item.take();
            self.slots_min.fill_slot_opt(item);

            need_replacement = true;
        }

        // right subtree
        if decision.traverse_right || self.open_slots() || need_replacement {
            need_replacement = self.descend_right(drv, idx, need_replacement);
        }

        // this node again
        if need_replacement {
            // this node was consumed, and both subtrees are empty now: nothing more to do here
            node.height = 0;
        } else {
            if self.open_max_slots() {
                // this node is the maximum of the tree: use it to fulfill a max request
//                self.move_to_slot(idx, ItemListId::MAX);
                let item = node.item.take();
                self.slots_max.fill_slot_opt(item);

                // fulfill the remaining max requests from the left subtree
                need_replacement = if self.tree.has_left(idx) {
                    debug_assert!(self.slots_min.is_empty());
                    self.descend_left(&mut RejectDriver, idx, true)
                } else {
                    true
                }
            }

            // update height
            if need_replacement {
                node.height = 0;
            } else {
                self.tree.update_height(idx);
            }
        }
    }


    //---- descend_* -------------------------------------------------------------------------------
    /// Returns true if the item needs replacement after recursive call, false otherwise.
    #[inline(always)]
    fn descend_left<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize,
                                           push_slot: bool) -> bool {
        if push_slot {
            debug_assert!(self.node(idx).item.is_none());
            self.slots_max.push_slot()
        }

        self.delete_range_recurse(drv, ImplicitTree::<T>::lefti(idx));

        // TODO: we do not handle correctly the case where after return from recursion there are some open min_reqs.
        // That is because it's a case that doesn't happen with range queries.

        if push_slot {
            let slot = self.slots_max.pop();

            if slot.is_some() {
                debug_assert!(self.node(idx).item.is_none());
                self.node_mut(idx).item = slot;
                false
            } else {
                true
            }
        } else {
            false
        }
//        if push_slot {
////            let node = self.node_mut(idx);
//            debug_assert!(self.node(idx).item.is_none());
//            self.slots_max.push_slot();
//
//            self.delete_range_recurse(drv, ImplicitTree::<T>::lefti(idx));
//
//            // TODO: we do not handle correctly the case where after return from recursion there are some open slots_min.
//            // That is because it's a case that doesn't happen with range queries.
//            let slot = self.slots_max.pop();
//
//            if slot.is_some() {
//                debug_assert!(self.node(idx).item.is_none());
//                self.node_mut(idx).item = slot;
//                false
//            } else {
//                true
//            }
//        } else {
//            self.delete_range_recurse(drv, ImplicitTree::<T>::lefti(idx));
//            false
//        }
    }

    /// Returns true if the item needs replacement after recursive call, false otherwise.
    #[inline(always)]
    fn descend_right<D: TraversalDriver<T>>(&mut self, drv: &mut D, idx: usize,
                                            push_slot: bool) -> bool {
        if push_slot {
            debug_assert!(self.node(idx).item.is_none());
            self.slots_min.push_slot()
        }

        self.delete_range_recurse(drv, ImplicitTree::<T>::righti(idx));

        if push_slot {
            let slot = self.slots_min.pop();

            if slot.is_some() {
                debug_assert!(self.node(idx).item.is_none());
                self.node_mut(idx).item = slot;
                false
            } else {
                true
            }
        } else {
            false
        }
//        if push_slot {
////            let node = self.node_mut(idx);
//            debug_assert!(self.node(idx).item.is_none());
//            self.slots_min.push_slot();
//
//            self.delete_range_recurse(drv, ImplicitTree::<T>::righti(idx));
//
//            let slot = self.slots_min.pop();
//
//            if slot.is_some() {
//                debug_assert!(self.node(idx).item.is_none());
//                self.node_mut(idx).item = slot;
//                false
//            } else {
//                true
//            }
//        } else {
//            self.delete_range_recurse(drv, ImplicitTree::<T>::righti(idx));
//            false
//        }
    }

//    #[inline]
//    fn take_max_left(&mut self, idx: usize, max_reqs: usize) {
//        self.descend_left(&mut RejectDriver, idx, 0, max_reqs);
//    }


    //---- helpers ---------------------------------------------------------------------------------
    #[inline(always)]
    fn open_slots(&self) -> bool {
        self.open_min_slots() || self.open_max_slots()
    }

    #[inline(always)]
    fn open_min_slots(&self) -> bool {
//        if let Some(slot) = self.slots_min.last() {
//            slot.is_some()
//        } else {
//            false
//        }
        self.slots_min.nslots() != self.slots_min.nfilled()
    }

    #[inline(always)]
    fn open_max_slots(&self) -> bool {
//        if let Some(slot) = self.slots_max.last() {
//            slot.is_some()
//        } else {
//            false
//        }
        self.slots_max.nslots() != self.slots_max.nfilled()
    }

    // Assumes that the returned vec will never be realloc'd!
    #[inline(always)]
    fn pin_stack(stack: &mut SlotStack<T>) -> SlotStack<T> {
        let nslots = stack.nslots;
        let slots = {
//            let mut v = &mut stack.slots;
//            let ptr = (&mut v[nslots..]).as_mut_ptr();
            let ptr = stack.slot_at(nslots) as *mut T;
            unsafe {
//                Vec::from_raw_parts(ptr, v.len()-nslots, v.capacity() - nslots)
                SlotStack {
                    nslots: 0,
                    nfilled: 0,
                    slots: Unique::new(ptr),
                    capacity: stack.capacity - nslots
                }
            }
        };

        slots
    }


//    #[inline]
//    fn move_to_slot(&mut self, idx: usize, list_id: ItemListId) -> &mut Node<T> {
//        let node = self.tree.node_mut(idx);
//        let item = node.item.take();
//        let mut stack = match list_id {
//            ItemListId::MIN => &mut self.slots_min,
//            ItemListId::MAX => &mut self.slots_max,
////            ItemListId::OUT => &mut self.output
//        };
//        debug_assert!(stack.nfilled() < stack.nslots()); // there should be no dynamic allocation here! we pre-allocate enough space in all 3 vecs
//        assert!(!stack.is_empty());
////        *stack.last_mut().unwrap() = item;
//        stack.fill_slot_opt(item);
//        node
//    }

//    #[inline]
//    fn move_to_slot(&mut self, item: &mut Option<T>, stack: &mut SlotStack<T>) -> &mut Node<T>{
//        debug_assert!(stack.len() < stack.capacity()); // there should be no dynamic allocation here! we pre-allocate enough space in all 3 vecs
//        *stack.last_mut().unwrap() = item.take();
//    }

    #[inline(always)]
    fn node(&self, idx: usize) -> &Node<T> {
        self.tree.node(idx)
    }

    #[inline(always)]
    fn node_mut<'b>(&mut self, idx: usize) -> &'b mut Node<T> {
        unsafe {
            mem::transmute(self.tree.node_mut(idx))
        }
    }

    #[inline(always)]
    fn item(&mut self, idx: usize) -> &Option<T> {
        &self.node(idx).item
    }

    #[inline(always)]
    fn item_mut(&mut self, idx: usize) -> &mut Option<T> {
        &mut self.node_mut(idx).item
    }
}


impl <T: Item> Drop for SlotStack<T> {
    fn drop(&mut self) {
        unsafe {
            let slots_vec = Vec::from_raw_parts(self.slots.get_mut(), self.nfilled, self.capacity);
            // let it drop
        }
    }
}