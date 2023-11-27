//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next, TaskInfo2, TaskStatus},
    task::get_cur_taskinfo,
    timer::get_time_us,
    timer::get_time_ms, syscall::{SYSCALL_WRITE, SYSCALL_EXIT, SYSCALL_YIELD, SYSCALL_GET_TIME, SYSCALL_TASK_INFO}, config::MAX_SYSCALL_NUM,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}
#[derive(Copy, Clone)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
}
/// Task information

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let tmp: TaskInfo2 = get_cur_taskinfo();
    let mut tmp1 = TaskInfo{
        time: get_time_ms() - tmp.time,
        syscall_times: [0u32; MAX_SYSCALL_NUM],
        status: tmp.status,
    };
    tmp1.syscall_times[SYSCALL_WRITE] = tmp.syscall_times[0];
    tmp1.syscall_times[SYSCALL_EXIT] = tmp.syscall_times[1];
    tmp1.syscall_times[SYSCALL_YIELD] = tmp.syscall_times[2];
    tmp1.syscall_times[SYSCALL_GET_TIME] = tmp.syscall_times[3];
    tmp1.syscall_times[SYSCALL_TASK_INFO] = tmp.syscall_times[4];
    assert!(tmp1.syscall_times[SYSCALL_TASK_INFO] > 0);
    unsafe{
       *ti = tmp1;
       assert!((*ti).syscall_times[SYSCALL_TASK_INFO] > 0);
    }
    0
}
