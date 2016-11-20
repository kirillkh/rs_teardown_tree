use std::mem;
use std::cmp::max;
use delete_bulk::{DeleteBulk, TraversalDriver, TraversalDecision};

pub trait Item: Sized {
    type Key: Ord;

    fn ord(&self) -> Self::Key;
}


//pub type Item = Sized+Ord;

#[derive(Debug)]
pub struct Node<T: Item> {
    pub item: Option<T>,    // TODO we can remove the option and use height==0 as null indicator
//    pub max: T::Key,
    pub height: u32,
}

#[derive(Debug)]
pub struct ImplicitIntervalTree<T: Item> {
    data: Vec<Node<T>>,
    size: usize,
}


impl<T: Item> ImplicitIntervalTree<T> {
    pub fn new(sorted: Vec<T>) -> ImplicitIntervalTree<T> {
        let size = sorted.len();
        let default = Node::<T>{item: None, height: 0};

        let capacity = Self::level_from(size)*4 + 3;

        let mut data = Vec::with_capacity(capacity);
        for i in 0..capacity {
            data.push(Node{item: None, height: 0});
        }

        let mut sorted: Vec<Option<T>> = sorted.into_iter().map(|x| Some(x)).collect();
        Self::build(&mut sorted, 0, &mut data);
        ImplicitIntervalTree { data: data, size: size }
    }


    fn build(sorted: &mut [Option<T>], idx: usize, data: &mut [Node<T>]) {
        match sorted.len() {
            0 => {}
            n => {
                let mid = n/2;
                let (lefti, righti) = (Self::lefti(idx), Self::righti(idx));
                Self::build(&mut sorted[..mid], lefti, data);
                Self::build(&mut sorted[mid+1..], righti, data);

                let height = 1 + max(data[lefti].height, data[righti].height);
                data[idx] = Node { item: sorted[mid].take(), height: height };
            }
        }
    }


    pub fn len(&self) -> usize {
        self.data.len()
    }



    pub fn delete_bulk<D: TraversalDriver<T>>(&mut self, drv: &mut D) -> Vec<T> {
        let output = {
            let mut d = DeleteBulk::new(self);
            d.delete_bulk(drv);
            d.output
        };
        self.size -= output.len();
        output
    }




    fn delete_idx(&mut self, mut idx: usize) -> T {
        let removed = self.delete_idx_recursive(idx);
        // update the parents
        while idx != 0 {
            idx = Self::parenti(idx);
            self.update_height(idx);
        }
        self.size -= 1;

        removed
    }


    pub fn node(&self, idx: usize) -> &Node<T> {
        &self.data[idx]
    }

    pub fn node_mut(&mut self, idx: usize) -> &mut Node<T> {
        &mut self.data[idx]
    }


    pub fn item(&self, idx: usize) -> &T {
        self.node(idx).item.as_ref().unwrap()
    }

    pub fn item_mut(&mut self, idx: usize) -> &mut T {
        self.node_mut(idx).item.as_mut().unwrap()
    }


    fn delete_idx_recursive(&mut self, idx: usize) -> T {
        assert!(idx >= 0 && !self.is_null(idx));

        if !self.has_left(idx) && !self.has_right(idx) {
            //            if idx != 0 {
            //                let parent = self.parent_mut(idx);
            //                parent.has_child[Self::branch(idx)] = false;
            //            }
            let root = self.node_mut(idx);
            root.height = 0;
            root.item.take().unwrap()
        } else {
            let removed = if self.has_left(idx) && !self.has_right(idx) {
                let left_max = self.delete_max(Self::lefti(idx));
                mem::replace(self.item_mut(idx), left_max)
            } else if !self.has_left(idx) && self.has_right(idx) {
                let right_min = self.delete_min(Self::righti(idx));
                mem::replace(self.item_mut(idx), right_min)
            } else { // self.has_left(idx) && self.has_right(idx)
                // TODO: remove from the subtree with bigger height, not always from the left
                let left_max = self.delete_max(Self::lefti(idx));
                mem::replace(self.item_mut(idx), left_max)
            };

            self.update_height(idx);
            removed
        }
    }


    #[inline]
    pub fn update_height(&mut self, idx: usize) {
        let h = max(self.left(idx).height, self.right(idx).height) + 1;
        let node = self.node_mut(idx);
        assert!(node.item.is_some());
        node.height =  h;
    }


    fn delete_max(&mut self, idx: usize) -> T {
        // TODO: rewrite with loop
        if self.has_right(idx) {
            let removed = self.delete_max(Self::righti(idx));
            self.update_height(idx);
            removed
        } else {
            // this is the max, now just need to handle the left subtree
            self.delete_idx_recursive(idx)
        }
    }

    fn delete_min(&mut self, idx: usize) -> T {
        // TODO: rewrite with loop
        if self.has_left(idx) {
            let removed = self.delete_min(Self::lefti(idx));
            self.update_height(idx);
            removed
        } else {
            // this is the min, now just need to handle the right subtree
            self.delete_idx_recursive(idx)
        }
    }


    //    #[inline]
    //    fn levels_count(&self) -> usize {
    //        if self.data.is_empty() {
    //            0
    //        } else {
    //            Self::level_of(self.data.len()-1) + 1
    //        }
    //    }

    #[inline]
    fn level_from(level: usize) -> usize {
        (1 << level) - 1
    }

    #[inline]
    fn level_of(idx: usize) -> usize {
        mem::size_of::<usize>()*8 - ((idx+1).leading_zeros() as usize) - 1
    }

    #[inline]
    fn row_start(idx: usize) -> usize {
        Self::level_from(Self::level_of(idx))
    }


    #[inline]
    pub fn parenti(idx: usize) -> usize {
        (idx-1) >> 1
    }

    #[inline]
    pub fn lefti(idx: usize) -> usize {
        (idx<<1) + 1
    }

    #[inline]
    pub fn righti(idx: usize) -> usize {
        (idx<<1) + 2
    }


    #[inline]
    pub fn parent(&self, idx: usize) -> &Node<T> {
        &self.data[Self::parenti(idx)]
    }

    #[inline]
    pub fn left(&self, idx: usize) -> &Node<T> {
        &self.data[Self::lefti(idx)]
    }

    #[inline]
    pub fn right(&self, idx: usize) -> &Node<T> {
        &self.data[Self::righti(idx)]
    }


    #[inline]
    pub fn parent_mut(&mut self, idx: usize) -> &mut Node<T> {
        &mut self.data[Self::parenti(idx)]
    }

    #[inline]
    pub fn left_mut(&mut self, idx: usize) -> &mut Node<T> {
        &mut self.data[Self::lefti(idx)]
    }

    #[inline]
    pub fn right_mut(&mut self, idx: usize) -> &mut Node<T> {
        &mut self.data[Self::righti(idx)]
    }



    #[inline]
    pub fn has_left(&self, idx: usize) -> bool {
        self.left(idx).height != 0
    }

    #[inline]
    pub fn has_right(&self, idx: usize) -> bool {
        self.right(idx).height != 0
    }

    #[inline]
    pub fn is_null(&self, idx: usize) -> bool {
        self.data[idx].item.is_none()
    }









    //    fn delete_bulk<P: TraversalPredicate<Node>>(&mut self, pred: P) {
    //        let height = self.data[0].height;
    //        let replacements_min = Vec::with_capacity(height);
    //        let replacements_max = Vec::with_capacity(height);
    //        let output = Vec::with_capacity(self.data.len);
    //
    //        self.delete_bulk_recursive(pred, 0,
    //                                   0, 0,
    //                                   &mut replacements_min, &mut replacements_max,
    //                                   &mut output)
    //    }
    //
    //
    //    #[inline]
    //    fn delete_bulk_leaf<P: TraversalPredicate<Node>>(&mut self, mut pred: P, idx: usize,
    //                                                     mut replace_min: usize, mut replace_max: usize,
    //                                                     replacements_min: &mut Vec<T>, replacements_max: &mut Vec<T>,
    //                                                     output: &mut Vec<T>) {
    //        let decision = pred.decide(self.node_mut(idx));
    //
    //        let mut replace_min_requested = (replace_min != replacements_min.len());
    //        let mut replace_max_requested = (replace_max != replacements_max.len());
    //
    //        // this element is the max and the min of its subtree
    //        if decision.consume_curr || replace_min_requested || replace_max_requested {
    //            // remove the element and push it to one of the output vecs
    //            let push_to =
    //                if decision.consume_curr { output }
    //                // TODO: does it matter that we always prioritize replacements_min?
    //                else if replace_min_requested { replacements_min }
    //                    else { replacements_max };
    //
    //            let elem = self.node_mut(idx);
    //            let item = elem.item.take().unwrap();
    //            push_to.push(item);
    //
    //            elem.height -= 1;
    //            assert!(elem.height == 0);
    //        } // else nothing to do
    //    }
    //
    //
    //    fn delete_bulk_left<P: TraversalPredicate<Node>>(&mut self, mut pred: P, idx: usize,
    //                                                     mut replace_min: usize, mut replace_max: usize,
    //                                                     replacements_min: &mut Vec<T>, replacements_max: &mut Vec<T>,
    //                                                     output: &mut Vec<T>) {
    //        let decision = pred.decide(self.node_mut(idx));
    //
    //        let mut replace_min_requested = (replace_min != replacements_min.len());
    //        let mut replace_max_requested = (replace_max != replacements_max.len());
    //
    //        // this element is the max of its subtree
    //        let mut remove_curr = (decision.consume_curr || replace_max_requested);
    //        if remove_curr {
    //            let push_to =
    //            if decision.consume_curr {
    //                replace_max_requested = true;
    //                output
    //            }
    //                else {
    //                    replacements_max
    //                };
    //
    //            let node = self.node_mut(idx);
    //            let item = node.item.take().unwrap();
    //            push_to.push(item);
    //
    //            assert!(node.height > 1);
    //
    //            replace_max += 1;
    //        }
    //
    //        assert!(!decision.traverse_right);
    //
    //        if decision.traverse_left || replace_min_requested || replace_max_requested {
    //            self.delete_bulk_recursive(pred, Self::lefti(idx),
    //                                       replace_min, replace_max,
    //                                       replacements_min, replacements_max,
    //                                       output);
    //
    //            replace_min_requested = (replace_min != replacements_min.len());
    //            replace_max_requested = (replace_max != replacements_max.len());
    //            if replace_min_requested || replace_max_requested {
    //                // the left subtree didn't have enough elements to satisfy all open replacement requests
    //                let node = self.node_mut(idx);
    //                if remove_curr {
    //                    // the node has already been removed above
    //                } else {
    //                    // this node has become a leaf, and we are going to remove it as well
    //                    let push_to =
    //                    // TODO: does it matter that we always prioritize replacements_min?
    //                    if replace_min_requested { replacements_min }
    //                        else { replacements_max };
    //
    //                    let item = node.item.take().unwrap();
    //                    push_to.push(item);
    //
    //                    assert!(!self.has_left(idx));
    //                }
    //
    //                node.height = 0;
    //            } else {
    //                if remove_curr {
    //                    // replace the current node (which has been removed above) by one from stack
    //                    assert!(self.node_mut(idx).item.is_none());
    //                    let item = replace_max.pop().unwrap();
    //                    self.node_mut(idx).item = Some(item);
    //                } // else nothing to do
    //
    //                self.update_height(idx);
    //            }
    //        } // else nothing to do
    //    }
    //
    //
    //    // mirrors delete_bulk_left() above
    //    fn delete_bulk_right<P: TraversalPredicate<Node>>(&mut self, mut pred: P, idx: usize,
    //                                                      mut replace_min: usize, mut replace_max: usize,
    //                                                      replacements_min: &mut Vec<T>, replacements_max: &mut Vec<T>,
    //                                                      output: &mut Vec<T>) {
    //        let decision = pred.decide(self.node_mut(idx));
    //
    //        let mut replace_min_requested = (replace_min != replacements_min.len());
    //        let mut replace_max_requested = (replace_max != replacements_max.len());
    //
    //        // this element is the min of its subtree
    //        let mut remove_curr = (decision.consume_curr || replace_min_requested);
    //        if remove_curr {
    //            let push_to =
    //            if decision.consume_curr {
    //                replace_min_requested = true;
    //                output
    //            }
    //                else {
    //                    replacements_min
    //                };
    //
    //            let node = self.node_mut(idx);
    //            let item = node.item.take().unwrap();
    //            push_to.push(item);
    //
    //            assert!(node.height > 0);
    //
    //            replace_max += 1;
    //        }
    //
    //        assert!(!decision.traverse_left);
    //
    //        if decision.traverse_right || replace_min_requested || replace_max_requested {
    //            self.delete_bulk_recursive(pred, Self::righti(idx),
    //                                       replace_min, replace_max,
    //                                       replacements_min, replacements_max,
    //                                       output);
    //
    //            replace_min_requested = (replace_min != replacements_min.len());
    //            replace_max_requested = (replace_max != replacements_max.len());
    //            if replace_min_requested || replace_max_requested {
    //                // the right subtree didn't have enough elements to satisfy all open replacement requests
    //                let node = self.node_mut(idx);
    //                if remove_curr {
    //                    // the node has already been removed above
    //                } else {
    //                    // this node has become a leaf, and we are going to remove it as well
    //                    let push_to =
    //                    // TODO: does it matter that we always prioritize replacements_max?
    //                    if replace_max_requested { replacements_max }
    //                        else { replacements_min };
    //
    //                    let item = node.item.take().unwrap();
    //                    push_to.push(item);
    //
    //                    assert!(!self.has_right(idx));
    //                }
    //
    //                node.height = 0;
    //            } else {
    //                if remove_curr {
    //                    // replace the current node (which has been removed above) by one from stack
    //                    assert!(self.node_mut(idx).item.is_none());
    //                    let item = replace_min.pop().unwrap();
    //                    self.node_mut(idx).item = Some(item);
    //                } // else nothing to do
    //
    //                self.update_height(idx);
    //            }
    //        } // else nothing to do
    //    }
    //
    //
    //
    //
    //
    //    fn delete_bulk_recurse_left<P: TraversalPredicate<Node>>(&mut self, mut pred: P, idx: usize,
    //                                                             mut replace_min: usize, mut replace_max: usize,
    //                                                             replacements_min: &mut Vec<T>, replacements_max: &mut Vec<T>,
    //                                                             output: &mut Vec<T>, decision: TraversalDecision) {
    //
    //        let mut replace_min_requested = (replace_min != replacements_min.len());
    //        let mut replace_max_requested = (replace_max != replacements_max.len());
    //        if decision.traverse_left || replace_min_requested || replace_max_left == 1 {
    //            self.delete_bulk_recursive(pred, Self::lefti(idx),
    //                                       replace_min, replace_max_left,
    //                                       replacements_min, &mut replacements_max_left,
    //                                       output);
    //
    //        }
    //    }
    //
    //
    //
    //
    //
    //    fn delete_bulk_both<P: TraversalPredicate<Node>>(&mut self, mut pred: P, idx: usize,
    //                                                     mut replace_min: usize, mut replace_max: usize,
    //                                                     replacements_min: &mut Vec<T>, replacements_max: &mut Vec<T>,
    //                                                     output: &mut Vec<T>) {
    //        let decision = pred.decide(self.node_mut(idx));
    //
    //        let mut replace_min_requested = (replace_min != replacements_min.len());
    //        let mut replace_max_requested = (replace_max != replacements_max.len());
    //
    //        // this element is not the min or the max of its subtree
    //        let replace_max_left = if decision.consume_curr {
    //            let node = self.node_mut(idx);
    //            let item = node.item.take().unwrap();
    //            output.push(item);
    //            1
    //        } else {
    //            0
    //        };
    //
    //        if decision.traverse_left || replace_min_requested || replace_max_left == 1 {
    //            let mut replacements_max_left = Self::subvec(replacements_max, replacements_max.len);
    //            self.delete_bulk_recursive(pred, Self::lefti(idx),
    //                                       replace_min, replace_max_left,
    //                                       replacements_min, &mut replacements_max_left,
    //                                       output);
    //
    //            replace_min_requested = (replace_min != replacements_min.len());
    //            replace_max_requested = (replace_max_left != replacements_max_left.len());
    //
    //            // The main idea here is, based on a case analysis:
    //            //   a) when the left subtree is not empty after the delete_bulk_recursive(node.left) call, we proceed to delete_bulk_recursive(node.right)
    //            //   b) when the left subtree is empty, we make sure the tree is in the correct shape to hand it over to delete_bulk_right(node)
    //
    //            // first deal with the possible replacing element for the current node in replacements_max_left
    //            // to get this whole hack out of the way
    //            if replacements_max_left.len == 1 {
    //                assert!(decision.consume_curr);
    //                let replacement = replacements_max_left.pop();
    //                self.node_mut(idx).item = Some(replacement);
    //            } else {
    //                assert!(replacements_max_left.len == 0);
    //            }
    //            // don't forget to dispose of it
    //            mem::forget(replacements_max_left);
    //
    //            //            // now, make sure that node.item.is_some()
    //            //            if replace_max_requested {
    //            //                // we know that the last item in replacements_min is the max item from our former left subtree (which is now empty)
    //            //                assert!(!self.has_left(idx));
    //            //                assert!(self.node_mut(idx).item.is_none());
    //            //                let replacement = replacements_min.pop();
    //            //                self.node_mut(idx).item = Some(replacement);
    //            //            }
    //
    //            if replace_min_requested || replace_max_requested {
    //                if replace_max_requested {
    //                    // left subtree empty, node removed
    //                    assert!(decision.consume_curr);
    //                    replace_min += 1;
    //                    self.delete_bulk_recursive(pred, Self::righti(idx),
    //                                               replace_min, replace_max,
    //                                               replacements_min, replacements_max,
    //                                               output);
    //                    replace_min_requested = (replace_min != replacements_min.len());
    //                    replace_max_requested = (replace_max != replacements_max.len());
    //
    //                    if replace_min_requested {
    //                        // right subtree empty, no replacement for the node
    //                        assert!(!self.has_left && !self.has_right);
    //                        self.node_mut(idx).height = 0;
    //                    } else {
    //                        // right subtree empty or non-empty, there is a replacement for the node
    //                        let replacement = replacements_min.pop();
    //                        let node = self.node_mut(idx);
    //
    //                        if replace_max_requested {
    //                            // right subtree empty, we must pass the item intended for this node higher up
    //                            replacements_max.push(replacement);
    //                            node.height = 0;
    //
    //                        } else {
    //                            // we don't need to pass the replacement up, so assign it to the node's item
    //                            node.item = Some(replacement);
    //                            node.height = 1 + self.node(Self::righti(idx)).height;
    //                        }
    //                    }
    //                } else {
    //                    // left subtree empty, node present
    //                    self.delete_bulk_recursive(pred, Self::righti(idx),
    //                                               replace_min, replace_max,
    //                                               replacements_min, replacements_max,
    //                                               output);
    //                    replace_min_requested = (replace_min != replacements_min.len());
    //                    replace_max_requested = (replace_max != replacements_max.len());
    //
    //                    if replace_min_requested || replace_max_requested {
    //                        // the right subtree didn't have enough elements to satisfy all open replacement requests
    //                        let node = self.node_mut(idx);
    //                        // this node has become a leaf, and we are going to remove it as well
    //                        let push_to =
    //                        // TODO: does it matter that we always prioritize replacements_max?
    //                        if replace_max_requested { replacements_max }
    //                            else { replacements_min };
    //
    //                        let item = node.item.take().unwrap();
    //                        push_to.push(item);
    //
    //                        assert!(!self.has_right(idx));
    //
    //                        node.height = 0;
    //                    } else {
    //                        self.update_height(idx);
    //                    }
    //                }
    //            } else {
    //                // left subtree might be empty or non-empty, node present
    //
    //            }
    //
    //
    //
    //
    //
    //
    //
    //
    //            if replace_min_requested || replace_max_requested {
    //                // the left subtree didn't have enough elements to satisfy all open replacement requests
    //                let node = self.node_mut(idx);
    //                if decision.consume_curr {
    //                    // the node has already been removed above:
    //
    //                    // restore replace_max to the old value and increment replace_min for the right traversal
    //                    replace_max -= 1;
    //                    replace_min += 1;
    //                    replace_min_requested = true;
    //                } else {
    //                    // this node has become a leaf, and we are going to remove it as well
    //                    let push_to =
    //                    // TODO: does it matter that we always prioritize replacements_min?
    //                    if replace_min_requested { replacements_min }
    //                        else { replacements_max };
    //
    //                    let item = node.item.take().unwrap();
    //                    push_to.push(item);
    //
    //                    assert!(!self.has_left(idx));
    //                }
    //
    //                node.height = 0;
    //            } else {
    //                if remove_curr {
    //                    // replace the current node (which has been removed above) by one from stack
    //                    assert!(self.node_mut(idx).item.is_none());
    //                    let item = replace_max.pop().unwrap();
    //                    self.node_mut(idx).item = Some(item);
    //                } // else nothing to do
    //
    //                self.update_height(idx);
    //            }
    //        } // else nothing to do
    //    }
    //
    //
    //    #[inline]
    //    fn delete_bulk_recursive<P: TraversalPredicate<Node>>(&mut self, mut pred: P, idx: usize,
    //                                                          mut replace_min: usize, mut replace_max: usize,
    //                                                          replacements_min: &mut Vec<T>, replacements_max: &mut Vec<T>,
    //                                                          output: &mut Vec<T>) {
    //        let decision = pred.decide(self.node_mut(idx));
    //
    //        let mut replace_min_requested = (replace_min != replacements_min.len());
    //        let mut replace_max_requested = (replace_max != replacements_max.len());
    //
    //        match (self.has_left(idx), self.has_right(idx)) {
    //            (false, false) => {
    //                self.delete_bulk_leaf(self, pred, idx, replace_min, replace_max, replacements_min, replacements_max, output);
    //            }
    //
    //            (true, false) => {
    //                self.delete_bulk_left(self, pred, idx, replace_min, replace_max, replacements_min, replacements_max, output);
    //            }
    //
    //            (false, true) => {
    //                self.delete_bulk_right(self, pred, idx, replace_min, replace_max, replacements_min, replacements_max, output);
    //            }
    //
    //            (true, true) => {
    //                self.delete_bulk_both(self, pred, idx, replace_min, replace_max, replacements_min, replacements_max, output);
    //            }
    //        }
    //    }
}
