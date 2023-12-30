//! Semaphore

use crate::sync::UPSafeCell;

use crate::task::current_process;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::{collections::VecDeque, sync::Arc};

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub id: usize,
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize, id:usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            id,
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        self.release_alloc(self.id);
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                let other_tid = task.inner_exclusive_access().res.as_ref().unwrap().tid;
                self.release_alloc(self.id);
                wakeup_task(task);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self)  {
        trace!("kernel: Semaphore::down");
        self.add_need(self.id);
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        }
        else{
            drop(inner);
        }
        self.need_into_alloc(self.id);
    }
    pub fn test(&self) -> isize{
        self.inner.exclusive_access().count
    }
    
    fn add_need(&self, id:usize) {
        let current_task_id = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        let current_process = current_process();
        let inner = current_process.inner_exclusive_access();
        inner.tasks[current_task_id].as_ref().unwrap().inner_exclusive_access().semaphore_need[id] += 1;
    }
    fn need_into_alloc(&self, id:usize) {
        let current_task_id = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        let current_process = current_process();
        let inner = current_process.inner_exclusive_access();
        inner.tasks[current_task_id].as_ref().unwrap().inner_exclusive_access().semaphore_need[id] -= 1;
        inner.tasks[current_task_id].as_ref().unwrap().inner_exclusive_access().semaphore_alloc[id] += 1;
    }
    fn release_alloc(&self, id:usize) {
        let current_task_id = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        let current_process = current_process();
        let inner = current_process.inner_exclusive_access();
        inner.tasks[current_task_id].as_ref().unwrap().inner_exclusive_access().semaphore_alloc[id] -= 1;
    }
}
