mod base;
mod delete_bulk;







#[cfg(test)]
mod tests {
    use base::{Item, ImplicitIntervalTree, Node};
    use delete_bulk::{TraversalDecision, TraversalDriver};


    impl Item for usize {
        type Key = usize;

        fn ord(&self) -> Self::Key {
            *self
        }
    }


    struct DriverFromTo {
        from: usize,
        to: usize
    }

    impl DriverFromTo {
        pub fn new(from: usize, to: usize) -> DriverFromTo {
            DriverFromTo { from:from, to:to }
        }
    }

    impl TraversalDriver<usize> for DriverFromTo {
        fn decide(&mut self, node: &mut Node<usize>) -> TraversalDecision {
            let x = node.item.unwrap();
            let left = self.from <= x;
            let right = x <= self.to;
            let consume = left && right;

            TraversalDecision { traverse_left: left, traverse_right: right, consume_curr: consume }
        }
    }


    #[test]
    fn build() {
        let tree = ImplicitIntervalTree::<usize>::new(vec![1]);
        let tree = ImplicitIntervalTree::<usize>::new(vec![1, 2]);
        let tree = ImplicitIntervalTree::<usize>::new(vec![1, 2, 3]);
        let tree = ImplicitIntervalTree::<usize>::new(vec![1, 2, 3, 4]);
        let tree = ImplicitIntervalTree::<usize>::new(vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn delete_bulk() {
        let mut tree = ImplicitIntervalTree::<usize>::new(vec![1, 2, 3]);
        let mut drv = DriverFromTo::new(3, 3);
        let d = tree.delete_bulk(&mut drv);
        println!("tree: {:?}", &tree);
        println!("output: {:?}", &d);
        panic!();
    }
}
