//! File and filesystem-related syscalls
use crate::fs::OSInode;
use crate::fs::{open_file, OpenFlags, Stat, StatMode, inode::ROOT_INODE, File};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer, PageTable, PhysAddr, VirtAddr};
use crate::task::{current_task, current_user_token};

use alloc::sync::Arc;

const PAGE_SIZE_BITS: usize = 12;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {   
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}
fn virt_addr_to_phys_addr(pt: &PageTable, vaddr: VirtAddr) -> PhysAddr{
    let vpn = vaddr.floor();
    let pte = pt.find_pte(vpn).unwrap();
    let ppn = pte.ppn();
    let paddr = PhysAddr::from(((usize::from(ppn)) << PAGE_SIZE_BITS) + vaddr.page_offset());
    paddr
}
/// YOUR JOB: Implement fstat.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    trace!(
        "kernel:pid[{}] sys_fstat",
        current_task().unwrap().pid.0
    );
    let task = current_task().unwrap();
    let token = task.get_user_token();
    let page_table = PageTable::from_token(token);
    //let tmp = current_task().unwrap();
    let inner = task.inner_nomut_access();
    if let Some(inode) = &inner.fd_table[fd] {
            let mut mode: StatMode = StatMode::NULL;
            
            if inode.is_file() {
                mode = StatMode::FILE;
            }
            
            //assert!(mode == StatMode::FILE);

            if inode.is_dir() {
                mode = StatMode::DIR;
            }

            
            let inode_id = inode.inode_id() as u64;
            
            let ref_cnt = inode.get_ref_cnt();
            /*let stat: Stat = Stat{
                dev: 0,
                ino: inode.is_dir(),
                mode,
                nlink: inode.get_ref_cnt(),
                pad: [0u64; 7],
            };*/
            

            unsafe{
                let dev_vaddr = VirtAddr::from(&((*st).dev) as *const u64 as usize);
                let ino_vaddr = VirtAddr::from(&((*st).ino) as *const u64 as usize);
                let mode_vaddr = VirtAddr::from(&((*st).mode) as *const StatMode as usize);
                let nlink_vaddr = VirtAddr::from(&((*st).nlink) as *const u32 as usize);
                
                let dev_paddr = virt_addr_to_phys_addr(&page_table, dev_vaddr).0 as *mut u64;
                let ino_paddr = virt_addr_to_phys_addr(&page_table, ino_vaddr).0 as *mut u64;
                let mode_paddr = virt_addr_to_phys_addr(&page_table, mode_vaddr).0 as *mut StatMode;
                let nlink_paddr = virt_addr_to_phys_addr(&page_table, nlink_vaddr).0 as *mut u32;
                
                *dev_paddr = 0u64;
                *ino_paddr = inode_id;
                *mode_paddr = mode;
                *nlink_paddr = ref_cnt;
            }
        return 0;
    }
    else{
        trace!("fail");
        return -1;
    } 
    return 0;
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_linkat",
        current_task().unwrap().pid.0
    );
    let token = current_task().unwrap().get_user_token();
    let old_name_str = translated_str(token, old_name);
    let new_name_str = translated_str(token, new_name);
    if old_name_str == new_name_str{
        return -1;
    }
    // let new_inode = ROOT_INODE.find(new_name_str.as_str());
    let old_inode = ROOT_INODE.find(old_name_str.as_str()).unwrap();
    
    ROOT_INODE.create_link(new_name_str.as_str(), &old_inode,old_name_str.as_str());

    0
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}
