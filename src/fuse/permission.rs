use std::{fs::File, io::{BufReader, BufRead}};

use crate::inode::inode;

pub const FILE_HANDLE_READ_BIT: u64 = 1 << 63;
pub const FILE_HANDLE_WRITE_BIT: u64 = 1 << 62;

pub fn as_file_kind(mut mode: u32) -> inode::InodeFileType {
    mode &= libc::S_IFMT as u32;
    if mode == libc::S_IFREG as u32 {
        return inode::InodeFileType::File;
    } else if mode == libc::S_IFDIR as u32 {
        return inode::InodeFileType::Directory;
    } else {
        unimplemented!("{}", mode);
    }
}

pub fn get_groups(pid: u32) -> Vec<u32> {
    {
        let path = format!("/proc/{}/task/{}/status", pid, pid);
        let file = File::open(path).unwrap();
        for line in BufReader::new(file).lines() {
            let line = line.unwrap();
            if line.starts_with("Groups:") {
                return line["Groups: ".len()..]
                    .split(' ')
                    .filter(|x| !x.trim().is_empty())
                    .map(|x| x.parse::<u32>().unwrap())
                    .collect();
            }
        }
    }
    vec![]
}

pub fn creation_gid(parent: &inode::InodeStat, gid: u32) -> u32 {
    if parent.mode & libc::S_ISGID as u16 != 0 {
        return parent.gid;
    }
    gid
}

pub fn creation_mode(mode: u32) -> u16 {
    (mode & !(libc::S_ISUID | libc::S_ISGID) as u32) as u16
}

pub fn check_file_handle_read(file_handle: u64) -> bool {
    (file_handle & FILE_HANDLE_READ_BIT) != 0
}

pub fn check_file_handle_write(file_handle: u64) -> bool {
    (file_handle & FILE_HANDLE_WRITE_BIT) != 0
}

pub fn check_access(
    file_uid: u32,
    file_gid: u32,
    file_mode: u16,
    uid: u32,
    gid: u32,
    mut access_mask: i32,
) -> bool {
    // F_OK tests for existence of file
    if access_mask == libc::F_OK {
        return true;
    }
    let file_mode = i32::from(file_mode);
    // root is allowed to read & write anything
    if uid == 0 {
        // root only allowed to exec if one of the X bits is set
        access_mask &= libc::X_OK;
        access_mask -= access_mask & (file_mode >> 6);
        access_mask -= access_mask & (file_mode >> 3);
        access_mask -= access_mask & file_mode;
        return access_mask == 0;
    }
    if uid == file_uid {
        access_mask -= access_mask & (file_mode >> 6);
    } else if gid == file_gid {
        access_mask -= access_mask & (file_mode >> 3);
    } else {
        access_mask -= access_mask & file_mode;
    }
    return access_mask == 0;
}