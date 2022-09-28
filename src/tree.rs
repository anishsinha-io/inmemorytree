#![allow(unused)]

use crate::sync::{Latch as _, Synchronized};

pub struct NodeInner<T> {
    keys: Vec<T>,
    children: Vec<Synchronized<NodeInner<T>>>,
    right_link: Option<Synchronized<NodeInner<T>>>,
    out_link: Option<Synchronized<NodeInner<T>>>,
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

    fn set_right_link(&mut self, right_link: Option<Synchronized<NodeInner<T>>>) {
        self.right_link = right_link;
    }

    fn set_out_link(&mut self, out_link: Option<Synchronized<NodeInner<T>>>) {
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
        assert!(!node.is_locked());
        let mut data = node.lock();
        assert!(node.is_locked());
        assert!(data.right_link.is_none());
        let sibling: Node<usize> = Node::create();
        data.right_link = Some(sibling);
        assert!(data.right_link.is_some());
        assert!(data.out_link.is_none());
        let out: Node<usize> = Node::create();
        data.out_link = Some(out);
        assert!(data.out_link.is_some());
    }

    /// This test uses latches
    #[test]
    fn test_create_and_set_sibling_and_out_with_latch() {
        let mut node: Node<usize> = Node::create();
        assert!(!node.is_locked());
        node.latch();
        assert!(node.is_locked());
        let inner = unsafe { &mut (*node.data_ptr()) };
        assert!(inner.right_link.is_none());
        let sibling: Node<usize> = Node::create();
        inner.set_right_link(Some(sibling));
        assert!(inner.right_link.is_some());
        let out: Node<usize> = Node::create();
        assert!(inner.out_link.is_none());
        inner.set_out_link(Some(out));
        assert!(inner.out_link.is_some());
        inner.set_out_link(None);
        assert!(inner.out_link.is_none());
        node.unlatch();
        assert!(!node.is_locked());
    }

    #[test]
    fn test_set_keys() {
        let mut node: Node<usize> = Node::create();
        assert!(!node.is_locked());
        node.latch();
        assert!(node.is_locked());
        let keys: Vec<usize> = vec![1, 2, 3, 4];
        let inner = unsafe { &mut (*node.data_ptr()) };
        assert!(inner.keys.len() == 0);
        inner.set_keys(keys);
        assert!(inner.keys.len() == 4);
        for i in 0..4 {
            assert!(inner.keys[i] == i + 1);
        }
        node.unlatch();
        assert!(!node.is_locked());
    }

    #[test]
    fn test_set_children() {
        let mut node: Node<usize> = Node::create();
        assert!(!node.is_locked());
        node.latch();
        assert!(node.is_locked());
        let inner = unsafe { &mut (*node.data_ptr()) };
        let keys: Vec<usize> = vec![1, 2, 3, 4];
        let child_one: Node<usize> = Node::create();
        let child_two: Node<usize> = Node::create();
        let child_three: Node<usize> = Node::create();
        let child_four: Node<usize> = Node::create();
        let child_five: Node<usize> = Node::create();
        let children: Vec<Node<usize>> =
            vec![child_one, child_two, child_three, child_four, child_five];
        inner.set_children(children);
        assert!(inner.children.len() == 5);
        node.unlatch();
        assert!(!node.is_locked());
    }
}
