#![allow(unused)]

use crate::sync::{Latch as _, Synchronized};

pub struct NodeInner<T> {
    right_link: Option<Synchronized<NodeInner<T>>>,
    out_link: Option<Synchronized<NodeInner<T>>>,
}

impl<T> NodeInner<T> {
    fn new() -> Self {
        Self {
            right_link: None,
            out_link: None,
        }
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
    #[test]
    fn test_create_and_set_sibling() {
        let mut node: Node<usize> = Node::create();
        let mut data = node.lock();
        assert!(data.right_link.is_none());
        let sibling: Node<usize> = Node::create();
        data.right_link = Some(sibling);
        assert!(data.right_link.is_some());
    }
}
