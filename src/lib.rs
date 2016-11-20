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
    fn delete_bulk4() {
        let mut tree = ImplicitIntervalTree::<usize>::new(vec![1, 2, 3, 4]);
        let mut drv = DriverFromTo::new(2,2);
        let d = tree.delete_bulk(&mut drv);
        println!("tree: {:?}", &tree);
        println!("output: {:?}", &d);
//        panic!();
    }


    fn test_prebuilt(items: &[usize], from_to: (usize, usize),
                     expect_tree: &[(usize, usize)], expect_out: &[usize]) {

        let nodes: Vec<Node<usize>> = mk_prebuilt(items);
        let mut tree = ImplicitIntervalTree::<usize>::with_nodes(nodes);
        let (from, to) = from_to;
        let mut drv = DriverFromTo::new(from, to);
        let output = tree.delete_bulk(&mut drv);
        assert_eq!(format!("{:?}", &tree), format!("{:?}", expect_tree));
        assert_eq!(format!("{:?}", &output), format!("{:?}", expect_out));
    }


    #[test]
    fn delete_bulk_prebuilt() {
        test_prebuilt(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4], (1,1),
                      &[(2, 3), (0, 0), (3, 2), (0, 0), (0, 0), (0, 0), (4, 1)], &[1]);

        test_prebuilt(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4], (2,2),
                      &[(1, 3), (0, 0), (3, 2), (0, 0), (0, 0), (0, 0), (4, 1)], &[2]);

        test_prebuilt(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4], (3,3),
                      &[(1, 3), (0, 0), (2, 2), (0, 0), (0, 0), (0, 0), (4, 1)], &[3]);

        test_prebuilt(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4], (4,4),
                      &[(1, 3), (0, 0), (2, 2), (0, 0), (0, 0), (0, 0), (3, 1)], &[4]);


        test_prebuilt(&[4, 3, 0, 2, 0, 0, 0, 1], (1,1),
                      &[(4, 3), (3, 2), (0, 0), (2, 1)], &[1]);

        test_prebuilt(&[4, 3, 0, 2, 0, 0, 0, 1], (2,2),
                      &[(4, 3), (3, 2), (0, 0), (1, 1)], &[2]);

        test_prebuilt(&[4, 3, 0, 2, 0, 0, 0, 1], (3,3),
                      &[(4, 3), (2, 2), (0, 0), (1, 1)], &[3]);

        test_prebuilt(&[4, 3, 0, 2, 0, 0, 0, 1], (4,4),
                      &[(3, 3), (2, 2), (0, 0), (1, 1)], &[4]);

    }


    fn mk_prebuilt(items: &[usize]) -> Vec<Node<usize>> {
        let mut nodes: Vec<_> = items.iter().map(|&x| if x==0 {
            Node { item:None, height: 0 }
        } else {
            Node { item: Some(x), height: 1}
        }).collect();

        for i in (1..items.len()).rev() {
            if nodes[i].height != 0 {
                let pari = ImplicitIntervalTree::<usize>::parenti(i);
                nodes[pari].height = ::std::cmp::max(nodes[pari].height, 1 + nodes[i].height);
            }
        }

        nodes
    }
}
