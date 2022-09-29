///----------------------------------------------------------------------------------------------------
/// The author disclaims copyright to this source code. In place of a legal notice, here is a blessing:
///     May you do good and not evil.
///     May you find forgiveness for yourself and forgive others.
///     May you share freely, never taking more than you give.
///----------------------------------------------------------------------------------------------------
/// This file implements synchronization primitives and methods for synchronization primitives used
/// throughout the implementation of the tree. Specifically, this file contains a correct
/// implementation of a binary semaphore as well as latch/unlatch methods for synchronized objects
/// (both protected by mutexes and protected by rwlocks). The mutexes are `parking_lot::Mutex` and
/// the rwlocks are `parking_lot::RwLock` (not std::sync::Mutex/std::sync::RwLock).
///----------------------------------------------------------------------------------------------------
use parking_lot::lock_api::{RawMutex as _, RawRwLock as _, RawRwLockUpgrade as _};
use parking_lot::{Condvar, Mutex, RwLock};
use std::sync::Arc;

/// BinarySemaphore: Semaphore with two states. Useful for setup tasks or making the main thread wait. Prefer using condvars if you're
/// trying to synchronize threads though.
pub type BinarySemaphore = Arc<(Mutex<bool>, Condvar)>;

/// Protect anything with a Mutex. Can pass between threads (implements the clone trait)
pub type Synchronized<T> = Arc<Mutex<T>>;

/// Protect anything with a RwLock. Can pass between threads (implements the clone trait).
pub type RwSynchronized<T> = Arc<RwLock<T>>;

/// Use this to specify the latch type
#[allow(unused)]
#[derive(PartialEq, Eq)]
pub enum LatchType {
    Shared,
    Upgradable,
    Excl,
}

/// Additional methods for Binary Semaphores
pub trait BinarySemaphoreMethods {
    fn init(state: bool) -> Self;
    fn post(&self);
    fn wait(&self) -> bool;
}

/// Additional methods for Synchronized<T> objects
pub trait Latch<T> {
    fn init(item: T) -> Self;
    fn latch(&self);
    fn unlatch(&self);
}

/// Additional methods for RwSynchronized<T> objects
pub trait RwLatch<T> {
    fn init(item: T) -> Self;
    fn acquire_shared(&self);
    fn acquire_upgradable(&self);
    fn acquire_excl(&self);
    fn release_shared(&self);
    fn release_upgradable(&self);
    fn release_excl(&self);
    fn upgrade_shared(&self);
}

/// Implement most of the POSIX Semaphore API (init/post/wait) but not value
impl BinarySemaphoreMethods for BinarySemaphore {
    fn init(state: bool) -> Self {
        Arc::new((Mutex::new(state), Condvar::new()))
    }

    fn post(&self) {
        let (mutex, condvar) = &**self;
        let mut state = mutex.lock();
        *state = !*state;
        condvar.notify_one();
    }

    fn wait(&self) -> bool {
        let (mutex, condvar) = &**self;
        let mut state = mutex.lock();
        while !*state {
            condvar.wait(&mut state);
        }
        *state
    }
}

/// The methods here are for latching Synchronized<T> objects *unsafely*. Don't use this unless you have to (prefer RAII guards)
/// Examples of when you need to use these methods:
/// - If you need to place a lock on an object in one function and unlock it in another function (i.e. when you can't do everything you)
///   want in one scope.
impl<T> Latch<T> for Synchronized<T> {
    fn init(item: T) -> Self {
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

/// The methods here are for latching RwSynchronized<T> objects *unsafely*. Don't use this unless you have to (prefer RAII guards)
/// Examples of when you need to use these methods:
/// - If you need to place a lock on an object in one function and unlock it in another function (i.e. when you can't do everything you)
///   want in one scope.
impl<T> RwLatch<T> for RwSynchronized<T> {
    fn init(item: T) -> Self {
        Arc::new(RwLock::new(item))
    }

    /// Acquire a shared lock. Must not hold a lock in the current context.
    fn acquire_shared(&self) {
        unsafe {
            self.raw().lock_shared();
        }
    }

    /// Acquire an upgradable lock. Must not hold a lock in the current context.
    fn acquire_upgradable(&self) {
        unsafe {
            self.raw().lock_upgradable();
        }
    }

    /// Acquire an exclusive lock. Must not hold a lock in the current context.
    fn acquire_excl(&self) {
        unsafe {
            self.raw().lock_exclusive();
        }
    }

    /// Release a shared lock. Must hold a shared lock in the current context.
    fn release_shared(&self) {
        unsafe {
            self.raw().unlock_shared();
        }
    }

    /// Release an upgradable lock. Must hold an upgradable lock in the current context.
    fn release_upgradable(&self) {
        unsafe {
            self.raw().unlock_upgradable();
        }
    }

    /// Release an exclusive lock. Must hold an exclusive lock in the current context (upgradable locks upgraded to exclusive qualify).
    fn release_excl(&self) {
        unsafe {
            self.raw().unlock_exclusive();
        }
    }

    /// Upgrade an upgradable lock to an exclusive one. Must hold an upgradable lock in the current context that has not yet been
    /// upgraded
    fn upgrade_shared(&self) {
        unsafe {
            self.raw().upgrade();
        }
    }
}

#[cfg(test)]
mod tests {
    use rayon::ThreadPoolBuilder;

    use super::{
        BinarySemaphore, BinarySemaphoreMethods as _, Latch as _, RwLatch as _, RwSynchronized,
        Synchronized,
    };
    struct TestStruct {
        data: usize,
    }

    fn check_thread(sync: &Synchronized<TestStruct>, sem: &BinarySemaphore) {
        loop {
            sync.latch();
            let inner = sync.data_ptr();
            let x = unsafe { (*inner).data };
            if x > 50 {
                sem.post();
            }
            sync.unlatch();
        }
    }

    fn increment_thread(sync: &Synchronized<TestStruct>) {
        loop {
            sync.latch();
            let inner = sync.data_ptr();
            unsafe { (*inner).data += 1 };
            sync.unlatch();
        }
    }

    #[test]
    fn test_binary_semaphore_and_latch() {
        let sem = BinarySemaphore::init(false);
        let sync_struct = Synchronized::init(TestStruct { data: 0 });
        let pool = ThreadPoolBuilder::new().num_threads(8).build().unwrap();
        for i in 0..8 {
            let sync_struct = sync_struct.clone();
            let sem = sem.clone();
            if i & 1 == 0 {
                pool.spawn(move || check_thread(&sync_struct, &sem));
            } else {
                pool.spawn(move || increment_thread(&sync_struct));
            }
        }
        let state = sem.wait();
        assert!(state == true);
        assert!(unsafe { (*sync_struct.data_ptr()).data } > 50);
    }

    fn rw_determine(rw_sync: &RwSynchronized<TestStruct>) -> bool {
        unsafe {
            rw_sync.acquire_upgradable();
            (*rw_sync.data_ptr()).data % 3 == 0
        }
    }

    fn rw_check_thread(rw_sync: &RwSynchronized<TestStruct>, sem: &BinarySemaphore) {
        loop {
            let inner = rw_sync.read();
            if inner.data > 50 {
                sem.post()
            }
        }
    }

    fn rw_maybe_increment_thread(rw_sync: &RwSynchronized<TestStruct>) {
        loop {
            if rw_determine(&rw_sync) {
                rw_sync.upgrade_shared();
                unsafe {
                    (*rw_sync.data_ptr()).data += 5;
                }
                rw_sync.release_excl();
                return;
            }
            rw_sync.release_upgradable();
        }
    }

    fn rw_increment_thread(rw_sync: &RwSynchronized<TestStruct>) {
        loop {
            let mut ts = rw_sync.write();
            ts.data += 1;
        }
    }

    #[test]
    fn test_rwlatch() {
        let sem = BinarySemaphore::init(false);
        let rw_sync_struct = RwSynchronized::init(TestStruct { data: 0 });
        let pool = ThreadPoolBuilder::new().num_threads(20).build().unwrap();
        for i in 0..20 {
            let rw_sync_struct = rw_sync_struct.clone();
            let sem = sem.clone();
            if i < 5 {
                pool.spawn(move || rw_check_thread(&rw_sync_struct, &sem));
            } else if i < 10 {
                pool.spawn(move || rw_maybe_increment_thread(&rw_sync_struct));
            } else {
                pool.spawn(move || rw_increment_thread(&rw_sync_struct));
            }
        }
        let state = sem.wait();
        assert!(state == true);
        assert!(unsafe { (*rw_sync_struct.data_ptr()).data } > 50);
    }
}
