//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    config::MAX_SYSCALL_NUM,
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
        change_program_brk, TaskInfo2, get_cur_task_info,
    }, timer::get_time_ms, mm::{PhysAddr, VirtAddr, VirtPageNum, PhysPageNum}, mm::PageTable 
};

use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

const SYSCALL_WRITE: usize = 64;
/// exit syscall
const SYSCALL_EXIT: usize = 93;
/// yield syscall
const SYSCALL_YIELD: usize = 124;
/// gettime syscall
const SYSCALL_GET_TIME: usize = 169;
/// sbrk syscall
const SYSCALL_SBRK: usize = 214;
/// munmap syscall
const SYSCALL_MUNMAP: usize = 215;
/// mmap syscall
const SYSCALL_MMAP: usize = 222;
/// taskinfo syscall
const SYSCALL_TASK_INFO: usize = 410;

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

fn virt_addr_to_phys_addr(pt: &PageTable, vaddr: VirtAddr) -> PhysAddr{
    let vpn = vaddr.floor();
    let pte = pt.find_pte(vpn).unwrap();
    let ppn = pte.ppn();
    let paddr = PhysAddr::from(((usize::from(ppn)) << PAGE_SIZE_BITS) + vaddr.page_offset());
    //println!("vaddr {} paddr {} pageoff {} ppageoff {}",usize::from(vaddr), usize::from(paddr), vaddr.page_offset(), paddr.page_offset());
    paddr
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let time = get_time_ms();
    assert!(time != 0);
    let ret: TimeVal = TimeVal { sec: time/1000, usec: time%1000 };
    let cur_page_table = PageTable::from_token(current_user_token());
    unsafe{
        //println!("time ptr {}", ts as usize);
        let sec_vaddr = VirtAddr::from(&((*ts).sec)as *const usize as usize);
        let usec_vaddr =  VirtAddr::from((&(*ts).usec)as *const usize as usize);
        
        //println!("sec ptr {}", usize::from(sec_vaddr));
        //println!("usec ptr {}", usize::from(usec_vaddr));
        let sec_paddr = virt_addr_to_phys_addr(&cur_page_table, sec_vaddr).0 as *mut usize;
        let usec_paddr = virt_addr_to_phys_addr(&cur_page_table, usec_vaddr).0 as *mut usize;
        
        //println!("sec paddr {}", sec_paddr as usize);
        //println!("usec paddr {}", usec_paddr as usize);
        *sec_paddr = ret.sec;
        *usec_paddr = ret.usec;
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let task_info = get_cur_task_info();
    let run_time = get_time_ms() - task_info.start_time;
    /*ret.syscall_times[SYSCALL_WRITE] = task_info.syscall_times[0];
    ret.syscall_times[SYSCALL_EXIT] = task_info.syscall_times[1];
    ret.syscall_times[SYSCALL_YIELD] = task_info.syscall_times[2];
    ret.syscall_times[SYSCALL_GET_TIME] = task_info.syscall_times[3];
    ret.syscall_times[SYSCALL_TASK_INFO] = task_info.syscall_times[4];
    ret.syscall_times[SYSCALL_MMAP] = task_info.syscall_times[5];
    ret.syscall_times[SYSCALL_MUNMAP] = task_info.syscall_times[6];
    ret.syscall_times[SYSCALL_SBRK] = task_info.syscall_times[7];*/
    let cur_page_table = PageTable::from_token(current_user_token());
    unsafe{
        let status_vaddr = VirtAddr::from(&((*ti).status)as *const TaskStatus as usize);
        let syscall_times_vaddr = VirtAddr::from(&((*ti).syscall_times[0])as *const u32 as usize);
        let time_vaddr = VirtAddr::from(&((*ti).time)as *const usize as usize);
        let status_paddr = virt_addr_to_phys_addr(&cur_page_table, status_vaddr).0;
        let status_ptr = status_paddr as *mut TaskStatus;
        *status_ptr = TaskStatus::Running;
        let mut i:usize = 0;
        while i < 500 {
            let syscall_times_paddr = virt_addr_to_phys_addr(&cur_page_table, VirtAddr::from(usize::from(syscall_times_vaddr) + i * 4)).0;
            let cur_syscall_times_paddr = syscall_times_paddr as *mut u32;
            match i {
                SYSCALL_WRITE => *cur_syscall_times_paddr = task_info.syscall_times[0],
                SYSCALL_EXIT => *cur_syscall_times_paddr = task_info.syscall_times[1],
                SYSCALL_YIELD => *cur_syscall_times_paddr = task_info.syscall_times[2],
                SYSCALL_GET_TIME => *cur_syscall_times_paddr = task_info.syscall_times[3],
                SYSCALL_TASK_INFO => *cur_syscall_times_paddr = task_info.syscall_times[4],
                SYSCALL_MMAP => *cur_syscall_times_paddr = task_info.syscall_times[5],
                SYSCALL_MUNMAP => *cur_syscall_times_paddr = task_info.syscall_times[6],
                SYSCALL_SBRK => *cur_syscall_times_paddr = task_info.syscall_times[7],
                _ => *cur_syscall_times_paddr = 0,
            };
            // *cur_syscall_times_paddr = ret.syscall_times[i / 4];
            i += 1;
        }
        let time_paddr = virt_addr_to_phys_addr(&cur_page_table, time_vaddr).0 as *mut usize;
        *time_paddr = run_time;
    }
    0
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
    -1
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}
