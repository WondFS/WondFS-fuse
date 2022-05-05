extern crate fuser;
use fuser::*;
use std::time::{UNIX_EPOCH, SystemTime, Duration};
use crate::inode::inode;

pub const FILE_HANDLE_READ_BIT: u64 = 1 << 63;
pub const FILE_HANDLE_WRITE_BIT: u64 = 1 << 62;

impl From<inode::InodeFileType> for fuser::FileType {
    fn from(kind: inode::InodeFileType) -> Self {
        match kind {
            inode::InodeFileType::File => fuser::FileType::RegularFile,
            inode::InodeFileType::Directory => fuser::FileType::Directory,
        }
    }
}

pub fn transfer_stat_to_attr(stat: inode::InodeStat) -> FileAttr {
    FileAttr {
        ino: stat.ino as u64,
        size: stat.size as u64,
        blocks: ((stat.size + 512 - 1) / 512) as u64,
        atime: system_time_from_time(stat.last_accessed.0, stat.last_accessed.1),
        mtime: system_time_from_time(stat.last_modified.0, stat.last_modified.1),
        ctime: system_time_from_time(
            stat.last_metadata_changed.0,
            stat.last_metadata_changed.1,
        ),
        kind: stat.file_type.into(),
        perm: stat.mode,
        nlink: stat.n_link as u32,
        uid: stat.uid,
        gid: stat.gid,
        rdev: 0,
        flags: 0,
        blksize: 512,
        padding: 0,
        crtime: SystemTime::UNIX_EPOCH,
    }
}

// pub fn transfer_attr_to_stat(attr: FileAttr) -> inode::InodeStat {
//     // inode::InodeStat {
//     //     file_type: attr.into(),
//     //     ino: attr.ino as u32,
//     //     size: 0,
//     //     uid: 0,
//     //     gid: 0,
//     //     ref_cnt: 0,
//     //     n_link: 0,
//     //     mode: todo!(),
//     //     last_accessed: todo!(),
//     //     last_modified: todo!(),
//     //     last_metadata_changed: todo!(),
//     // }
//     todo!()
// }

pub fn time_now() -> (i64, u32) {
    time_from_system_time(&SystemTime::now())
}

pub fn system_time_from_time(secs: i64, nsecs: u32) -> SystemTime {
    if secs >= 0 {
        UNIX_EPOCH + Duration::new(secs as u64, nsecs)
    } else {
        UNIX_EPOCH - Duration::new((-secs) as u64, nsecs)
    }
}

pub fn time_from_system_time(system_time: &SystemTime) -> (i64, u32) {
    match system_time.duration_since(UNIX_EPOCH) {
        Ok(duration) => (duration.as_secs() as i64, duration.subsec_nanos()),
        Err(before_epoch_error) => (
            -(before_epoch_error.duration().as_secs() as i64),
            before_epoch_error.duration().subsec_nanos(),
        ),
    }
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
