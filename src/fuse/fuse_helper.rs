extern crate fuser;
use fuser::*;
use std::{time::{UNIX_EPOCH, SystemTime, Duration}};
use crate::inode::inode;

/// Transfer InodeFileType to fuser FileType
/// params:
/// kind - InodeFileType
/// return:
/// fuser FileType
impl From<inode::InodeFileType> for fuser::FileType {
    fn from(kind: inode::InodeFileType) -> Self {
        match kind {
            inode::InodeFileType::File => fuser::FileType::RegularFile,
            inode::InodeFileType::Directory => fuser::FileType::Directory,
        }
    }
}

/// Transfer InodeStat to fuser FileAttr
/// params:
/// stat - InodeStat
/// return:
/// fuser FileAttr
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

/// Get time from now
/// params:
/// ()
/// return:
/// time
pub fn time_now() -> (i64, u32) {
    time_from_system_time(&SystemTime::now())
}

/// Get SystemTime from time
/// params:
/// secs - seconds
/// nsecs - additional nanoseconds
/// return:
/// SystemTime
pub fn system_time_from_time(secs: i64, nsecs: u32) -> SystemTime {
    if secs >= 0 {
        UNIX_EPOCH + Duration::new(secs as u64, nsecs)
    } else {
        UNIX_EPOCH - Duration::new((-secs) as u64, nsecs)
    }
}

/// Get time from SystemTime
/// params:
/// system_time - SystemTime
/// return:
/// time
pub fn time_from_system_time(system_time: &SystemTime) -> (i64, u32) {
    match system_time.duration_since(UNIX_EPOCH) {
        Ok(duration) => (duration.as_secs() as i64, duration.subsec_nanos()),
        Err(before_epoch_error) => (
            -(before_epoch_error.duration().as_secs() as i64),
            before_epoch_error.duration().subsec_nanos(),
        ),
    }
}