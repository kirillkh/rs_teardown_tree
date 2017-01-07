mod slot_stack;
mod bulk_delete;
mod unsafe_stack;
mod base_repr;
pub mod drivers;

pub use self::slot_stack::*;
pub use self::bulk_delete::*;
pub use self::unsafe_stack::*;
pub use self::drivers::*;
pub use self::base_repr::*;

use std::ptr;
use std::mem;
use std::cmp::{Ordering};


/// A fast way to refill the tree from a master copy; adds the requirement for T to implement Copy.
pub trait TeardownTreeRefill<T: Copy+Ord> {
    fn refill(&mut self, master: &Self);
}



impl<T: Copy+Ord> TeardownTreeRefill<T> for TreeWrapper<T> {
    fn refill(&mut self, master: &TreeWrapper<T>) {
        let len = self.data.len();
        debug_assert!(len == master.data.len());
        unsafe {
            ptr::copy_nonoverlapping(master.data.as_ptr(), self.data.as_mut_ptr(), len);
            ptr::copy_nonoverlapping(master.mask.as_ptr(), self.mask.as_mut_ptr(), len);
        }
        self.size = master.size;
    }
}


//impl<T: Clone+Item> TeardownTreeRefill<T> for TeardownTree<T> {
//    fn refill(&mut self, master: &TeardownTree<T>) {
//            let len = self.data().len();
//            debug_assert!(len == master.data.len());
//            self.drop_items();
//
//            for i in 0..master.size() {
//                if master.mask[i] {
//                    self.place(i, master.data[i].item.clone());
//                }
//            }
//    }
//}


pub trait TreeBase<T: Ord>: TreeReprAccess<T> {
    #[inline(always)]
    fn item_mut_unsafe<'b>(&mut self, idx: usize) -> &'b mut T {
        unsafe {
            mem::transmute(&mut self.node_mut(idx).item)
        }
    }

    #[inline(always)]
    fn item_mut(&mut self, idx: usize) -> &mut T {
        &mut self.node_mut(idx).item
    }

    #[inline(always)]
    fn item(&self, idx: usize) -> &T {
        &self.node(idx).item
    }


    /// Finds the item with the given key and returns it (or None).
    fn lookup(&self, search: &T) -> Option<&T> {
        self.index_of(search).map(|idx| self.item(idx))
    }

    fn index_of(&self, search: &T) -> Option<usize> {
        if self.data.is_empty() {
            return None;
        }

        let mut idx = 0;
        let mut key =
        if self.mask[idx] {
            self.item(idx)
        } else {
            return None;
        };

        loop {
            let ordering = search.cmp(&key);

            idx = match ordering {
                Ordering::Equal   => return Some(idx),
                Ordering::Less    => lefti(idx),
                Ordering::Greater => righti(idx),
            };

            if idx >= self.data.len() {
                return None;
            }

            if self.mask[idx] {
                key = self.item(idx);
            } else {
                return None;
            }
        }
    }


    #[inline]
    fn find_max(&mut self, mut idx: usize) -> usize {
        while self.has_right(idx) {
            idx = righti(idx);
        }
        idx
    }

    #[inline]
    fn find_min(&mut self, mut idx: usize) -> usize {
        while self.has_left(idx) {
            idx = lefti(idx);
        }
        idx
    }

    #[inline(always)]
    fn node(&self, idx: usize) -> &Node<T> {
        &self.data[idx]
    }

    #[inline(always)]
    fn node_mut(&mut self, idx: usize) -> &mut Node<T> {
        &mut self.data[idx]
    }

    #[inline(always)]
    fn node_opt(&self, idx: usize) -> Option<&Node<T>> {
        if self.is_nil(idx) { None } else { Some(self.node(idx)) }
    }

    #[inline(always)]
    fn parent_opt(&self, idx: usize) -> Option<&Node<T>> {
        if idx == 0 {
            None
        } else {
            Some(&self.data[parenti(idx)])
        }
    }

    #[inline(always)]
    fn left_opt(&self, idx: usize) -> Option<&Node<T>> {
        let lefti = lefti(idx);
        if self.is_nil(lefti) {
            None
        } else {
            Some(&self.data[lefti])
        }
    }

    #[inline(always)]
    fn right_opt(&self, idx: usize) -> Option<&Node<T>> {
        let righti = righti(idx);
        if self.is_nil(righti) {
            None
        } else {
            Some(&self.data[righti])
        }
    }


    #[inline(always)]
    fn parent(&self, idx: usize) -> &Node<T> {
        let parenti = parenti(idx);
        debug_assert!(idx > 0 && !self.is_nil(idx));
        &self.data[parenti]
    }

    #[inline(always)]
    fn left(&self, idx: usize) -> &Node<T> {
        let lefti = lefti(idx);
        debug_assert!(!self.is_nil(lefti));
        &self.data[lefti]
    }

    #[inline(always)]
    fn right(&self, idx: usize) -> &Node<T> {
        let righti = righti(idx);
        debug_assert!(!self.is_nil(righti));
        &self.data[righti]
    }


    #[inline(always)]
    fn has_left(&self, idx: usize) -> bool {
        !self.is_nil(lefti(idx))
    }

    #[inline(always)]
    fn has_right(&self, idx: usize) -> bool {
        !self.is_nil(righti(idx))
    }

    #[inline(always)]
    fn is_nil(&self, idx: usize) -> bool {
        idx >= self.data.len() || !unsafe { *self.mask.get_unchecked(idx) }
    }


    /// Returns the closest subtree A enclosing `idx`, such that A is the left child (or 0 if no such
    /// node is found). `idx` is considered to enclose itself, so we return `idx` if it is the left
    /// child.
    /// **Attention!** For efficiency reasons, idx and return value are both **1-based**.
    #[inline(always)]
    fn left_enclosing(idx: usize) -> usize {
        if idx & 1 == 0 {
            idx
        } else if idx & 2 == 0 {
            idx >> 1
        } else {
            let shift = (idx + 1).trailing_zeros();
            idx >> shift
        }
    }




    /// Returns the closest subtree A enclosing `idx`, such that A is the right child (or 0 if no such
    /// node is found). `idx` is considered to enclose itself, so we return `idx` if it is the right
    /// child.
    /// **Attention!** For efficiency reasons, idx and return value are both **1-based**.
    #[inline(always)]
    fn right_enclosing(idx: usize) -> usize {
        if idx & 1 == 1 {
            idx
        } else if idx & 2 == 1 {
            idx >> 1
        } else {
            let shift = idx.trailing_zeros();
            idx >> shift
        }
    }

    #[inline]
    fn traverse_preorder<A, F>(&mut self, root: usize, a: &mut A, mut f: F)
        where F: FnMut(&mut Self, &mut A, usize) {
        if self.is_nil(root) {
            return;
        }

        let mut next = root;

        loop {
            next = {
                f(self, a, next);

                if self.has_left(next) {
                    lefti(next)
                } else if self.has_right(next) {
                    righti(next)
                } else {
                    loop {
                        let l_enclosing = {
                            let z = next + 2;

                            if z == z & (!z+1) {
                                0
                            } else {
                                Self::left_enclosing(next+1)-1
                            }
                        };

                        if l_enclosing <= root {
                            // done
                            return;
                        }

                        next = l_enclosing + 1; // right sibling
                        if !self.is_nil(next) {
                            break;
                        }
                    }
                    next
                }
            };
        }
    }

    #[inline(never)]
    fn traverse_inorder<A, F>(&mut self, root: usize, a: &mut A, mut on_next: F)
                                                where F: FnMut(&mut Self, &mut A, usize) -> bool {
        if self.is_nil(root) {
            return;
        }

        let mut next = self.find_min(root);

        loop {
            next = {
                let stop = on_next(self, a, next);
                if stop {
                    break;
                }

                if self.has_right(next) {
                    self.find_min(righti(next))
                } else {
                    let l_enclosing = Self::left_enclosing(next+1);

                    if l_enclosing <= root+1 {
                        // done
                        break;
                    }

                    parenti(l_enclosing-1)
                }
            }
        }
    }


    #[inline]
    fn traverse_inorder_rev<A, F>(&mut self, root: usize, a: &mut A, mut f: F)
                                                        where F: FnMut(&mut Self, &mut A, usize) {
        if self.is_nil(root) {
            return;
        }

        let mut next = self.find_max(root);

        loop {
            next = {
                f(self, a, next);

                if self.has_left(next) {
                    self.find_max(lefti(next))
                } else {
                    let r_enclosing = Self::right_enclosing(next);

                    if r_enclosing <= root {
                        // done
                        return;
                    }

                    parenti(r_enclosing)
                }
            }
        }
    }

    #[inline(always)]
    fn take(&mut self, idx: usize) -> T {
        debug_assert!(!self.is_nil(idx), "idx={}, mask[idx]={}", idx, self.mask[idx]);
        let p: *const Node<T> = unsafe {
            self.data.get_unchecked(idx)
        };
        self.mask[idx] = false;
        self.size -= 1;
        unsafe { ptr::read(&(*p).item) }
    }

    #[inline(always)]
    unsafe fn move_to<S: Sink<T>>(&mut self, idx: usize, sink: &mut S) {
        debug_assert!(!self.is_nil(idx), "idx={}, mask[idx]={}", idx, self.mask[idx]);
        *self.mask.get_unchecked_mut(idx) = false;
        self.size -= 1;
        let p: *const Node<T> = self.data.get_unchecked(idx);

        let item = ptr::read(&(*p).item);
        sink.consume_unchecked(item);
    }

    #[inline(always)]
    unsafe fn move_from_to(&mut self, src: usize, dst: usize) {
        debug_assert!(!self.is_nil(src) && self.is_nil(dst), "is_nil(src)={}, is_nil(dst)={}", self.is_nil(src), self.is_nil(dst));
        *self.mask.get_unchecked_mut(src) = false;
        *self.mask.get_unchecked_mut(dst) = true;
        let pdata = self.data.as_mut_ptr();
        let psrc: *mut Node<T> = pdata.offset(src as isize);
        let pdst: *mut Node<T> = pdata.offset(dst as isize);
        let x = ptr::read(psrc);
        ptr::write(pdst, x);
    }

    #[inline(always)]
    fn place(&mut self, idx: usize, item: T) {
        if self.mask[idx] {
            self.data[idx].item = item;
        } else {
            self.mask[idx] = true;
            self.size += 1;
            unsafe {
                let p = self.data.get_unchecked_mut(idx);
                ptr::write(p, Node::new(item));
            };
        }
    }

    fn clear(&mut self) {
        self.drop_items();
    }

    fn drop_items(&mut self) {
        if self.size*2 <= self.data.len() {
            self.traverse_preorder(0, &mut 0, |this: &mut Self, _, idx| {
                unsafe {
                    let p = this.data.get_unchecked_mut(idx);
                    ptr::drop_in_place(p);
                }

                this.mask[idx] = false;
            })
        } else {
            let p = self.data.as_mut_ptr();
            for i in 0..self.size {
                if self.mask[i] {
                    unsafe {
                        ptr::drop_in_place(p.offset(i as isize));
                    }

                    self.mask[i] = false;
                }
            }
        }

        self.size = 0;
    }


    fn slots_min<'a>(&'a mut self) -> &'a mut SlotStack where T: 'a {
        &mut self.delete_range_cache.slots_min
    }

    fn slots_max<'a>(&'a mut self) -> &'a mut SlotStack where T: 'a {
        &mut self.delete_range_cache.slots_max
    }

    fn size(&self) -> usize {
        self.size
    }
}

impl<T: Ord> TreeBase<T> for TreeWrapper<T> {}


#[inline(always)]
pub fn parenti(idx: usize) -> usize {
    (idx-1) >> 1
}

#[inline(always)]
pub fn lefti(idx: usize) -> usize {
    (idx<<1) + 1
}

#[inline(always)]
pub fn righti(idx: usize) -> usize {
    (idx<<1) + 2
}




#[cfg(test)]
pub mod validation {
    use rand::{Rng, XorShiftRng, SeedableRng};
    use std::fmt::Debug;
    use base::{lefti, righti, parenti, TreeWrapper, TreeBase};

    /// Validates the BST property.
    pub fn check_bst<'a, T: Ord+Debug, U: Ord+Debug>(tree: &'a TreeWrapper<T>, output: &Vec<U>, tree_orig: &TreeWrapper<T>, idx: usize) -> Option<(&'a T, &'a T)> {
        if tree.size() == 0 || !tree.is_nil(idx) {
            return None;
        }

        let node = tree.node_opt(idx);
        if node.is_none() {
            return None;
        } else {
            let item = &node.unwrap().item;
            let left = check_bst(tree, output, tree_orig, lefti(idx));
            let right = check_bst(tree, output, tree_orig, righti(idx));

            let min =
                if let Some((lmin, lmax)) = left {
                    debug_assert!(lmax < item, "tree_orig: {:?}, tree: {:?}, output: {:?}", tree_orig, tree, output);
                    lmin
                } else {
                    item
                };
            let max =
                if let Some((rmin, rmax)) = right {
                    debug_assert!(item < rmin, "tree_orig: {:?}, tree: {:?}, output: {:?}", tree_orig, tree, output);
                    rmax
                } else {
                    item
                };

            return Some((min, max));
        }
    }

    /// Checks that there are no dangling items (the parent of every item marked as present is also marked as present).
    pub fn check_integrity<T: Ord+Debug>(tree: &TreeWrapper<T>, tree_orig: &TreeWrapper<T>) {
        let mut noccupied = 0;

        for i in 0..tree.data.len() {
            if tree.mask[i] {
                debug_assert!(i == 0 || tree.mask[parenti(i)], "tree_orig: {:?}, {}, tree: {:?}", tree_orig, tree_orig, tree);
                noccupied += 1;
            }
        }

        debug_assert!(noccupied == tree.size());
    }


    pub fn gen_tree_items<T: Ord+Clone>(items: Vec<T>) -> Vec<Option<T>> {
        let mut shaped = vec![None; 1 << 18];
        let mut rng = XorShiftRng::from_seed([3, 1, 4, 15]);
        gen_subtree_items(&items, 0, &mut shaped, &mut rng);

        let mut items = shaped.into_iter()
            .rev()
            .skip_while(|opt| opt.is_none())
            .collect::<Vec<_>>();
        items.reverse();
        items
    }

    fn gen_subtree_items<T: Ord+Clone>(items: &[T], idx: usize, output: &mut Vec<Option<T>>, rng: &mut XorShiftRng) {
        if items.len() == 0 {
            return;
        }

        // hack
        if idx >= output.len() {
            return;
        }

        let root = rng.gen_range(0, items.len());
        output[idx] = Some(items[root].clone());
        gen_subtree_items(&items[..root], lefti(idx), output, rng);
        gen_subtree_items(&items[root+1..], righti(idx), output, rng);
    }
}
