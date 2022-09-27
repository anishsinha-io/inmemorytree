use parking_lot::lock_api::{RawMutex as _, RawRwLock as _, RawRwLockUpgrade as _};
use parking_lot::{Condvar, Mutex, RwLock};
use std::sync::Arc;

pub type BinarySemaphore = Arc<(Mutex<bool>, Condvar)>;
pub type Synchronized<T> = Arc<Mutex<T>>;
pub type RwSynchronized<T> = Arc<RwLock<T>>;

pub trait BinarySemaphoreMethods {
    fn new(state: bool) -> Self;
    fn post(&self);
    fn value(&self);
    fn wait(&self);
}

pub trait Latch<T> {
    fn new(item: T) -> Self;
    fn latch(&self);
    fn unlatch(&self);
}

pub trait RwLatch<T> {
    fn new(item: T) -> Self;
    fn acquire_shared(&self);
    fn acquire_upgradable(&self);
    fn acquire_excl(&self);
    fn release_shared(&self);
    fn release_upgradable(&self);
    fn release_excl(&self);
    fn upgrade_shared(&self);
}

impl BinarySemaphoreMethods for BinarySemaphore {
    fn new(state: bool) -> Self {
        Arc::new((Mutex::new(state), Condvar::new()))
    }

    fn post(&self) {}

    fn value(&self) {}

    fn wait(&self) {}
}

impl<T> Latch<T> for Synchronized<T> {
    fn new(item: T) -> Self {
        Arc::new(Mutex::new(item))
    }
    fn latch(&self) {
        unsafe {
            self.raw().lock();
        }
    }
    fn unlatch(&self) {
        unsafe {
            self.raw().unlock();
        }
    }
}

impl<T> RwLatch<T> for RwSynchronized<T> {
    fn new(item: T) -> Self {
        Arc::new(RwLock::new(item))
    }

    fn acquire_shared(&self) {
        unsafe {
            self.raw().lock_shared();
        }
    }

    fn acquire_upgradable(&self) {
        unsafe {
            self.raw().lock_upgradable();
        }
    }

    fn acquire_excl(&self) {
        unsafe {
            self.raw().lock_exclusive();
        }
    }

    fn release_shared(&self) {
        unsafe {
            self.raw().unlock_shared();
        }
    }

    fn release_upgradable(&self) {
        unsafe {
            self.raw().unlock_upgradable();
        }
    }

    fn release_excl(&self) {
        unsafe {
            self.raw().unlock_exclusive();
        }
    }

    fn upgrade_shared(&self) {
        unsafe {
            self.raw().upgrade();
        }
    }
}

#[cfg(test)]
mod tests {}
