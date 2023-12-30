//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::task::current_process;
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(&self);
    /// Unlock the mutex
    fn unlock(&self);
    /// test mutex value
    fn test(&self) -> usize;
    /// add need for deadlock detection
    fn add_need(&self, id:usize);
    /// turn need into alloc for deadlock detection
    fn need_into_alloc(&self, id:usize);
    /// release alloc for deadlock detection
    fn release_alloc(&self, id:usize);
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    id: usize,
    inner: UPSafeCell<MutexSpinInner>
}

pub struct MutexSpinInner {
    pub locked: bool
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new(id:usize) -> Self {
        let tmp = MutexSpinInner {
            locked: false
        };
        Self { id, inner: unsafe { UPSafeCell::new(tmp) } }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) {
        trace!("kernel: MutexSpin::lock");
        self.add_need(self.id);
        loop {
            let mut inner = self.inner.exclusive_access();
            if inner.locked {
                drop(inner);
                suspend_current_and_run_next();
                continue;
            } else {
                inner.locked = true;
                self.need_into_alloc(self.id);
                return;
            }
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let mut inner = self.inner.exclusive_access();
        self.release_alloc(self.id);
        inner.locked = false;
    }
    fn test(&self) -> usize{
        !self.inner.exclusive_access().locked as usize
    }
    fn add_need(&self, id:usize) {
        let current_task_id = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        let current_process = current_process();
        let inner = current_process.inner_exclusive_access();
        inner.tasks[current_task_id].as_ref().unwrap().inner_exclusive_access().mutex_need[id] += 1;
    }
    fn need_into_alloc(&self, id:usize) {
        let current_task_id = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        let current_process = current_process();
        let inner = current_process.inner_exclusive_access();
        inner.tasks[current_task_id].as_ref().unwrap().inner_exclusive_access().mutex_need[id] -= 1;
        inner.tasks[current_task_id].as_ref().unwrap().inner_exclusive_access().mutex_alloc[id] += 1;
    }
    fn release_alloc(&self, id:usize) {
        let current_task_id = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        let current_process = current_process();
        let inner = current_process.inner_exclusive_access();
        inner.tasks[current_task_id].as_ref().unwrap().inner_exclusive_access().mutex_alloc[id] -= 1;
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    id: usize,
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new(id: usize) -> Self {
        trace!("kernel: MutexBlocking::new");
        let tmp = MutexBlockingInner {
            locked: false,
            wait_queue: VecDeque::new(),
        };
        Self {
            id,
            inner: unsafe {
                UPSafeCell::new(tmp)
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) {
        trace!("kernel: MutexBlocking::lock");
        self.add_need(self.id);
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
        }
        self.need_into_alloc(self.id);
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        self.release_alloc(self.id);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            let other_tid = waking_task.inner_exclusive_access().res.as_ref().unwrap().tid;
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
    fn test(&self) -> usize{
        !self.inner.exclusive_access().locked as usize
    }
    fn add_need(&self, id:usize) {
        let current_task_id = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        let current_process = current_process();
        let inner = current_process.inner_exclusive_access();
        inner.tasks[current_task_id].as_ref().unwrap().inner_exclusive_access().mutex_need[id] += 1;
    }
    fn need_into_alloc(&self, id:usize) {
        let current_task_id = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        let current_process = current_process();
        let inner = current_process.inner_exclusive_access();
        inner.tasks[current_task_id].as_ref().unwrap().inner_exclusive_access().mutex_need[id] -= 1;
        inner.tasks[current_task_id].as_ref().unwrap().inner_exclusive_access().mutex_alloc[id] += 1;
    }
    fn release_alloc(&self, id:usize) {
        let current_task_id = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        let current_process = current_process();
        let inner = current_process.inner_exclusive_access();
        inner.tasks[current_task_id].as_ref().unwrap().inner_exclusive_access().mutex_alloc[id] -= 1;
    }
}
