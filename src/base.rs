use std::mem;
use std::cmp::max;
use std::fmt::{Debug, Formatter};
use delete_bulk::{DeleteBulk, TraversalDriver};

pub trait Item: Sized+Clone+Debug {
    type Key: Ord+Debug;

    fn ord(&self) -> Self::Key;
}


//pub type Item = Sized+Ord;

#[derive(Debug, Clone)]
pub struct Node<T: Item> {
    pub item: Option<T>,    // TODO we can remove the option and use height==0 as null indicator
//    pub max: T::Key,
    pub height: u32,
}

#[derive(Clone)]
pub struct ImplicitIntervalTree<T: Item> {
    data: Vec<Node<T>>,
    size: usize,
}


impl<T: Item> ImplicitIntervalTree<T> {
    pub fn new(sorted: Vec<T>) -> ImplicitIntervalTree<T> {
        let size = sorted.len();

        let capacity = Self::level_from(size)*4 + 3;

        let mut data = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            data.push(Node{item: None, height: 0});
        }

        let mut sorted: Vec<Option<T>> = sorted.into_iter().map(|x| Some(x)).collect();
        Self::build(&mut sorted, 0, &mut data);
        ImplicitIntervalTree { data: data, size: size }
    }

    pub fn with_nodes(nodes: Vec<Node<T>>) -> ImplicitIntervalTree<T> {
        let size = nodes.iter().filter(|x| x.height != 0).count();
        let capacity = Self::level_from(size)*4 + 3; // allocate enough nodes that righti() is never out of bounds

        let mut data = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            data.push(Node{item: None, height: 0});
        }

        unsafe {
            use ::std::ptr;
            ptr::copy_nonoverlapping(nodes.as_ptr(), data.as_mut_ptr(), nodes.len());
        }
        ::std::mem::forget(nodes);

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
        assert!(!self.is_null(idx));

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

    pub fn size(&self) -> usize {
        self.size
    }
}


impl<T: Item> Debug for ImplicitIntervalTree<T> {
    fn fmt(&self, fmt: &mut Formatter) -> ::std::fmt::Result {
        let mut nz: Vec<_> = self.data.iter()
            .rev()
            .skip_while(|node| node.item.is_none() && node.height==0)
            .map(|node| match node.item {
                None => (String::from("0"), 0),
                Some(ref x) => (format!("{:?}", x.ord()), node.height)
            })
            .collect();
        nz.reverse();

        let _ = write!(fmt, "[");
        let mut sep = "";
        for &(ref key, ref height) in nz.iter() {
            let _ = write!(fmt, "{}", sep);
            sep = ", ";
            let _ = write!(fmt, "({}, {})", key, height);
        }
        let _ = write!(fmt, "]");
        Ok(())
    }
}
