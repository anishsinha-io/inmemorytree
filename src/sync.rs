use parking_lot::lock_api::{RawMutex as _, RawRwLock as _, RawRwLockUpgrade as _};
use parking_lot::{Condvar, Mutex, RwLock};
use std::sync::Arc;

pub type BinarySemaphore = Arc<(Mutex<bool>, Condvar)>;
pub type Synchronized<T> = Arc<Mutex<T>>;
pub type RwSynchronized<T> = Arc<RwLock<T>>;

pub trait BinarySemaphoreMethods {
    fn init(state: bool) -> Self;
    fn post(&self);
    fn wait(&self) -> bool;
}

pub trait Latch<T> {
    fn init(item: T) -> Self;
    fn latch(&self);
    fn unlatch(&self);
}

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

impl<T> RwLatch<T> for RwSynchronized<T> {
    fn init(item: T) -> Self {
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
mod tests {
    #![allow(unused)]
    use rayon::ThreadPoolBuilder;

    use super::{
        BinarySemaphore, BinarySemaphoreMethods as _, Latch as _, RwLatch as _, RwSynchronized,
        Synchronized,
    };
    struct TestStruct {
        data: usize,
    }

    impl TestStruct {
        fn new() -> Self {
            Self { data: 0 }
        }
    }

    fn check_thread(sync: &Synchronized<TestStruct>, sem: &BinarySemaphore) {
        loop {
            let (mutex, condvar) = &**sem;
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
        let mut sync_struct = Synchronized::init(TestStruct { data: 0 });
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
        let mut rw_sync_struct = RwSynchronized::init(TestStruct { data: 0 });
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
