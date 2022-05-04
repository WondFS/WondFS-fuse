extern crate fuser;
use fuser::*;
use std::time::{UNIX_EPOCH, SystemTime, Duration};
use crate::inode::inode;

pub fn transfer_stat_to_attr(stat: inode::InodeStat) -> FileAttr {
    FileAttr {
        ino: 1,
        size: 0,
        blocks: 0,
        atime: UNIX_EPOCH, // 1970-01-01 00:00:00
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
        kind: FileType::Directory,
        perm: 0o755,
        nlink: 2,
        uid: 501,
        gid: 20,
        rdev: 0,
        flags: 0,
        blksize: 512,
        padding: 0,
    }
}

pub fn transfer_attr_to_stat(attr: FileAttr) -> inode::InodeStat {
    inode::InodeStat {
        file_type: inode::InodeFileType::File,
        ino: 1,
        size: 0,
        uid: 0,
        gid: 0,
        ref_cnt: 0,
        n_link: 0,
    }
}

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