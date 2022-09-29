#![allow(unused)]

use crate::node::{BTreeNode as _, Node, NodeInner};
use crate::sync::{
    BinarySemaphore, BinarySemaphoreMethods as _, Latch as _, RwLatch as _, RwSynchronized,
    Synchronized,
};
pub struct Tree {}

pub trait BTree {}
