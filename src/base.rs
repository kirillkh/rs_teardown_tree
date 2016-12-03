use std::ptr;
use std::mem;
use std::cmp::{max, Ordering};
use std::fmt::{Debug, Formatter};
use delete_range::{DeleteRange, DeleteRangeCache, TraversalDriver, TraversalDecision};

pub trait Item: Sized+Clone {
    type Key: Ord;

    fn ord(&self) -> Self::Key;
}


impl Item for usize {
    type Key = usize;

    fn ord(&self) -> Self::Key {
        *self
    }
}


#[derive(Debug, Clone)]
pub struct Node<T: Item> {
    pub item: Option<T>,
}


/// A fast way to refill the tree from a master copy; adds the requirement for T to implement Copy.
pub trait TeardownTreeRefill<T: Copy+Item> {
    fn refill(&mut self, master: &TeardownTree<T>);
}


impl<T: Copy+Item> TeardownTreeRefill<T> for TeardownTree<T> {
    fn refill(&mut self, master: &TeardownTree<T>) {
        let len = self.data.len();
        debug_assert!(len == master.data.len());
        self.data.truncate(0);
        unsafe {
            ptr::copy_nonoverlapping(master.data.as_ptr(), self.data.as_mut_ptr(), len);
            self.data.set_len(len);
        }
        self.size = master.size;
    }
}


pub struct DriverFromTo {
    from: usize,
    to: usize
}

impl DriverFromTo {
    pub fn new(from: usize, to: usize) -> DriverFromTo {
        DriverFromTo { from:from, to:to }
    }
}

impl TraversalDriver<usize> for DriverFromTo {
    #[inline(always)]
    fn decide(&self, x: &usize) -> TraversalDecision {
        let left = self.from <= *x;
        let right = *x <= self.to;

        TraversalDecision { left: left, right: right }
    }
}






#[derive(Clone)]
pub struct TeardownTree<T: Item> {
    data: Vec<Node<T>>,
    size: usize,

    pub delete_range_cache: Option<DeleteRangeCache<T>>,
}

impl<T: Item> TeardownTree<T> {
    pub fn new(sorted: Vec<T>) -> TeardownTree<T> {
        let size = sorted.len();

        let capacity = size;

        let mut data = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            data.push(Node{item: None});
        }

        let mut sorted: Vec<Option<T>> = sorted.into_iter().map(|x| Some(x)).collect();
        let height = Self::build(&mut sorted, 0, &mut data);
        debug_assert!({ // we assert that the tree is nearly complete
            data.iter().take(sorted.len()).filter(|x| x.item.is_none()).count() == 0
        });
        let cache = DeleteRangeCache::new(height);
        TeardownTree { data: data, size: size, delete_range_cache: Some(cache) }
    }

    pub fn with_nodes(nodes: Vec<Node<T>>) -> TeardownTree<T> {
        let size = nodes.iter().filter(|x| x.item.is_some()).count();
        let height = Self::calc_height(&nodes, 0);
        let capacity = Self::row_start(nodes.len())*4 + 3; // allocate enough nodes that righti() is never out of bounds

        let mut data = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            data.push(Node{item: None});
        }

        unsafe {
            ptr::copy_nonoverlapping(nodes.as_ptr(), data.as_mut_ptr(), nodes.len());
        }
        ::std::mem::forget(nodes);

        let cache = DeleteRangeCache::new(height);
        TeardownTree { data: data, size: size, delete_range_cache: Some(cache) }
    }

    pub fn into_node_vec(self) -> Vec<Node<T>> {
        self.data
    }


    fn calc_height(nodes: &Vec<Node<T>>, idx: usize) -> usize {
        if idx < nodes.len() && nodes[idx].item.is_some() {
            1 + max(Self::calc_height(nodes, Self::lefti(idx)),
                    Self::calc_height(nodes, Self::righti(idx)))
        } else {
            0
        }
    }


    /// finds the point to partition n keys for a nearly-complete binary tree
    /// http://stackoverflow.com/a/26896494/3646645
    fn build_select_root(n: usize) -> usize {
        // the highest power of two <= n
        let x = if n.is_power_of_two() { n }
                else { n.next_power_of_two() / 2 };

        if x/2 <= (n-x) + 1 {
            debug_assert!(x >= 1, "x={}, n={}", x, n);
            x - 1
        } else {
            n - x / 2
        }
    }

    /// returns the height of the tree
    fn build(sorted: &mut [Option<T>], idx: usize, data: &mut [Node<T>]) -> usize {
        match sorted.len() {
            0 => 0,
            n => {
                let mid = Self::build_select_root(n);
                let (lefti, righti) = (Self::lefti(idx), Self::righti(idx));
                let lh = Self::build(&mut sorted[..mid], lefti, data);
                let rh = Self::build(&mut sorted[mid+1..], righti, data);

                data[idx] = Node { item: sorted[mid].take() };
                debug_assert!(rh <= lh);
                1 + lh
            }
        }
    }


    pub fn len(&self) -> usize {
        self.data.len()
    }



    pub fn delete_range<D: TraversalDriver<T>>(&mut self, drv: &mut D, output: &mut Vec<T>) {
        debug_assert!(output.is_empty());
        output.truncate(0);
        {
            DeleteRange::new(self, output).delete_range(drv);
            debug_assert!({
                let cache: DeleteRangeCache<T> = self.delete_range_cache.take().unwrap();
                let ok = cache.slots_min.is_empty() && cache.slots_max.is_empty() && cache.delete_subtree_stack.is_empty();
                self.delete_range_cache = Some(cache);
                ok
            });
        }
        self.size -= output.len();
    }




    #[inline(always)]
    pub fn node(&self, idx: usize) -> &Node<T> {
        &self.data[idx]
    }

    #[inline(always)]
    pub fn node_mut(&mut self, idx: usize) -> &mut Node<T> {
        &mut self.data[idx]
    }

    #[inline(always)]
    fn node_mut_unsafe<'b>(&mut self, idx: usize) -> &'b mut Node<T> {
        unsafe {
            mem::transmute(&mut self.data[idx])
        }
    }



    pub fn item_mut_unwrap(&mut self, idx: usize) -> &mut T {
        self.node_mut(idx).item.as_mut().unwrap()
    }


    pub fn delete(&mut self, search: &T) -> bool {
        let mut idx = 0;
        if idx >= self.data.len() {
            return false;
        }

        let mut ord =
            if let Some(ref it) = self.node_mut_unsafe(idx).item {
                it.ord()
            } else {
                return false;
            };

        loop {
            let ordering = search.ord().cmp(&ord);

            idx = match ordering {
                Ordering::Equal   => {
                    self.size -= 1;
                    self.delete_idx(idx);
                    return true
                },
                Ordering::Less    => Self::lefti(idx),
                Ordering::Greater => Self::righti(idx),
            };

            if idx >= self.data.len() {
                return false;
            }

            if let Some(ref it) = self.node_mut_unsafe(idx).item {
                ord = it.ord();
            } else {
                return false;
            }
        }
    }



    #[inline]
    fn delete_idx(&mut self, idx: usize) -> T {
        debug_assert!(!self.is_null(idx));

        match (self.has_left(idx), self.has_right(idx)) {
            (false, false) => {
                let root = self.node_mut(idx);
                root.item.take().unwrap()
            },

            (true, false)  => {
                let left_max = self.delete_max(Self::lefti(idx));
                mem::replace(self.item_mut_unwrap(idx), left_max)
            },

            (false, true)  => {
                let right_min = self.delete_min(Self::righti(idx));
                mem::replace(self.item_mut_unwrap(idx), right_min)
            },

            (true, true)   => {
                let left_max = self.delete_max(Self::lefti(idx));
                mem::replace(self.item_mut_unwrap(idx), left_max)
            },
        }
    }

    #[inline]
    fn delete_max(&mut self, mut idx: usize) -> T {
        while self.has_right(idx) {
            idx = Self::righti(idx);
        }

        if self.has_left(idx) {
            let left_max = self.delete_max(Self::lefti(idx));
            mem::replace(self.item_mut_unwrap(idx), left_max)
        } else {
            let root = self.node_mut(idx);
            root.item.take().unwrap()
        }
    }

    #[inline]
    fn delete_min(&mut self, mut idx: usize) -> T {
        while self.has_left(idx) {
            idx = Self::lefti(idx);
        }

        if self.has_right(idx) {
            let right_min = self.delete_min(Self::righti(idx));
            mem::replace(self.item_mut_unwrap(idx), right_min)
        } else {
            let root = self.node_mut(idx);
            root.item.take().unwrap()
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


    #[inline(always)]
    pub fn parent(&self, idx: usize) -> &Node<T> {
        &self.data[Self::parenti(idx)]
    }

    #[inline(always)]
    pub fn left(&self, idx: usize) -> Option<&Node<T>> {
        let lefti = Self::lefti(idx);
        if lefti < self.data.len() {
            Some(&self.data[lefti])
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn right(&self, idx: usize) -> Option<&Node<T>> {
        let righti = Self::righti(idx);
        if righti < self.data.len() {
            Some(&self.data[righti])
        } else {
            None
        }
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



    #[inline(always)]
    pub fn has_left(&self, idx: usize) -> bool {
        self.left(idx).and_then(|nd| nd.item.as_ref()).is_some()
    }

    #[inline(always)]
    pub fn has_right(&self, idx: usize) -> bool {
        self.right(idx).and_then(|nd| nd.item.as_ref()).is_some()
    }

    #[inline(always)]
    pub fn is_null(&self, idx: usize) -> bool {
        idx >= self.data.len() || self.data[idx].item.is_none()
    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.size
    }
}


impl<K: Ord+Debug, T: Item<Key=K>> Debug for TeardownTree<T> {
    fn fmt(&self, fmt: &mut Formatter) -> ::std::fmt::Result {
        let mut nz: Vec<_> = self.data.iter()
            .rev()
            .skip_while(|node| node.item.is_none())
            .map(|node| match node.item {
                None => String::from("0"),
                Some(ref x) => format!("{:?}", x.ord())
            })
            .collect();
        nz.reverse();

        let _ = write!(fmt, "[");
        let mut sep = "";
        for ref key in nz.iter() {
            let _ = write!(fmt, "{}", sep);
            sep = ", ";
            let _ = write!(fmt, "{}", key);
        }
        let _ = write!(fmt, "]");
        Ok(())
    }
}
