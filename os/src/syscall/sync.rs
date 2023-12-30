use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;

/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
fn deadlock_detect_mutex(mutex_id: usize) -> bool{
    // 每个mutex的数目
    let mut available:Vec<usize> = Vec::new();
    let cur_process = current_process();
    let inner = cur_process.inner_exclusive_access();
    let cur_task_id = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
    let task_cnt = inner.tasks.len();
    let mutex_cnt = inner.mutex_list.len();
    let mut finish: Vec<bool> = vec![false; task_cnt];
    let mut finish_cnt: usize = 0usize;
    for i in inner.mutex_list.iter(){
        available.push(i.as_ref().unwrap().test());
    }
    for (idx, i) in inner.tasks.iter().enumerate(){
        if i.is_none(){
            finish[idx] = true;
            finish_cnt += 1;
        }
    }
    for _ in 0..task_cnt{
        for (idx, i) in inner.tasks.iter().enumerate(){
            if !finish[idx]{
                let mut can_go_flag = true;
                for j in 0..mutex_cnt{
                    let mut cur_need = i.as_ref().unwrap().inner_exclusive_access().mutex_need[j];
                    if idx == cur_task_id && j == mutex_id{
                        cur_need += 1;
                    }
                    if available[j] < cur_need{
                        can_go_flag = false;
                        break;
                    }
                }
                if can_go_flag {
                    for j in 0..mutex_cnt{
                        available[j] += i.as_ref().unwrap().inner_exclusive_access().mutex_alloc[j];
                        finish[idx] = true;
                    }
                    finish_cnt += 1;
                    break;
                }
            }
        }
    }
    finish_cnt != task_cnt
}
fn deadlock_detect_semaphore(semaphore_id: usize) -> bool{
    let mut available:Vec<usize> = Vec::new();
    let cur_process = current_process();
    let inner = cur_process.inner_exclusive_access();
    let pid = cur_process.pid.0;
    let cur_task_id = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
    let task_cnt = inner.tasks.len();
    let semaphore_cnt = inner.semaphore_list.len();
    let mut finish: Vec<bool> = vec![false; task_cnt];
    let mut finish_cnt: usize = 0usize;
    for i in inner.semaphore_list.iter(){
        let tmp = i.as_ref().unwrap().test();
        available.push(if tmp < 0 {0usize} else {tmp as usize});
    }
    for (idx, i) in inner.tasks.iter().enumerate(){
        if i.is_none(){
            finish[idx] = true;
            finish_cnt += 1;
        }
    }
    for _ in 0..task_cnt{
        for (idx, i) in inner.tasks.iter().enumerate(){
            if !finish[idx]{
                let mut can_go_flag = true;
                for j in 0..semaphore_cnt{
                    let mut cur_need = i.as_ref().unwrap().inner_exclusive_access().semaphore_need[j];
                    if idx == cur_task_id && j == semaphore_id{
                        cur_need += 1;
                    }
                    if available[j] < cur_need{
                        can_go_flag = false;
                        break;
                    }
                }
                if can_go_flag {
                    for j in 0..semaphore_cnt{
                        let release = i.as_ref().unwrap().inner_exclusive_access().semaphore_alloc[j];
                        available[j] += release;
                        finish[idx] = true;
                    }
                    finish_cnt += 1;
                    break;
                }
            }
        }
    }
    finish_cnt != task_cnt
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        
        let mutex: Option<Arc<dyn Mutex>> = if !blocking {
            Some(Arc::new(MutexSpin::new(id)))
        } else {
            Some(Arc::new(MutexBlocking::new(id)))
        };
        process_inner.mutex_list[id] = mutex;
        for i in &process_inner.tasks{
            i.as_ref().unwrap().reset_mutex(id);
        }
        id as isize
    } else {
        let id = process_inner.mutex_list.len();
        let mutex: Option<Arc<dyn Mutex>> = if !blocking {
            Some(Arc::new(MutexSpin::new(id)))
        } else {
            Some(Arc::new(MutexBlocking::new(id)))
        };
        process_inner.mutex_list.push(mutex);
        for i in &process_inner.tasks{
            i.as_ref().unwrap().add_mutex();
        }
        id as isize
    }

}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let deadlock_detect = process_inner.deadlock_detect;
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    if deadlock_detect{
        if(deadlock_detect_mutex(mutex_id)){
            -0xDEAD
        }
        else{
            mutex.lock();
            0
        }
    }
    else{
        mutex.lock();
        0
    }
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count, id)));
        for i in &process_inner.tasks{
            i.as_ref().unwrap().reset_semaphore(id);
        }
        id
    } else {
        let id = process_inner.semaphore_list.len();
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count, id))));
        
        for i in &process_inner.tasks{
            i.as_ref().unwrap().add_semaphore();
        }
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let deadlock_detect = process_inner.deadlock_detect;
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    if deadlock_detect{
        if(deadlock_detect_semaphore(sem_id)){
            -0xDEAD
        }
        else{
            sem.down();
            0
        }
    }
    else{
        sem.down();
        0
    }
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect");
    match enabled {
        0 => current_process().inner_exclusive_access().deadlock_detect = false,
        1 => current_process().inner_exclusive_access().deadlock_detect = true,
        _ => return -1,
    };
    0
}
