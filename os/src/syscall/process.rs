//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus, TaskInfo2, get_cur_task_info, current_user_token
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
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
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

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
