#![allow(unused)]

use crate::sync::{Latch as _, Synchronized};

pub struct NodeInner<T> {
    keys: Vec<T>,
    children: Vec<Synchronized<NodeInner<T>>>,
    right_link: Option<*const Synchronized<NodeInner<T>>>,
    out_link: Option<*const Synchronized<NodeInner<T>>>,
}

impl<T> NodeInner<T> {
    fn new() -> Self {
        Self {
            keys: Vec::new(),
            children: Vec::new(),
            right_link: None,
            out_link: None,
        }
    }

    fn set_right_link(&mut self, right_link: Option<*const Synchronized<NodeInner<T>>>) {
        self.right_link = right_link;
    }

    fn set_out_link(&mut self, out_link: Option<*const Synchronized<NodeInner<T>>>) {
        self.out_link = out_link;
    }

    fn set_keys(&mut self, keys: Vec<T>) {
        self.keys = keys;
    }

    fn set_children(&mut self, children: Vec<Synchronized<NodeInner<T>>>) {
        self.children = children;
    }
}

pub type Node<T> = Synchronized<NodeInner<T>>;

pub trait BTreeNode<T> {
    fn create() -> Self;
}

impl<T> BTreeNode<T> for Node<T> {
    fn create() -> Self {
        Synchronized::init(NodeInner::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This test uses mutex guards which are technically safer, but more restrictive
    #[test]
    fn test_create_and_set_sibling_and_out() {
        let mut node: Node<usize> = Node::create();
        let mut data = node.lock();
        assert!(data.right_link.is_none());
        let sibling: Node<usize> = Node::create();
        data.right_link = Some(Synchronized::init(sibling).data_ptr());
        assert!(data.right_link.is_some());
        assert!(data.out_link.is_none());
        let out: Node<usize> = Node::create();
        data.out_link = Some(Synchronized::init(out).data_ptr());
        assert!(data.out_link.is_some());
    }

    /// This test uses latches
    #[test]
    fn test_create_and_set_sibling_and_out_with_latch() {
        let mut node: Node<usize> = Node::create();
        node.latch();
        let inner = unsafe { &mut (*node.data_ptr()) };
        assert!(inner.right_link.is_none());
        let sibling: Node<usize> = Node::create();
        inner.set_right_link(Some(Synchronized::init(sibling).data_ptr()));
        assert!(inner.right_link.is_some());
        let out: Node<usize> = Node::create();
        assert!(inner.out_link.is_none());
        inner.set_out_link(Some(Synchronized::init(out).data_ptr()));
        assert!(inner.out_link.is_some());
        inner.set_out_link(None);
        assert!(inner.out_link.is_none());
        node.unlatch();
    }

    #[test]
    fn test_set_keys() {
        let mut node: Node<usize> = Node::create();
        node.latch();
        let keys: Vec<usize> = vec![1, 2, 3, 4];
        let inner = unsafe { &mut (*node.data_ptr()) };
        assert!(inner.keys.len() == 0);
        inner.set_keys(keys);
        assert!(inner.keys.len() == 4);
        node.unlatch();
    }

    #[test]
    fn test_set_children() {
        let mut node: Node<usize> = Node::create();
        node.latch();
        let keys: Vec<usize> = vec![1, 2, 3, 4];

        node.unlatch();
    }
}
