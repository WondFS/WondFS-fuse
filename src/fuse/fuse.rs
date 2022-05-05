extern crate fuser;
extern crate libc;

use fuser::*;
use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;
use std::time::Duration;
use libc::ENOENT;
use std::sync::Arc;
use crate::inode::inode;
use crate::common::{file_table, path, directory};
use crate::fuse::fuse_helper::*;

const TTL: Duration = Duration::from_secs(1); // 1 second

pub struct WondFS {
    pub file: Vec<Option<file_table::FileLink>>,
    pub max_file: u32,
    pub file_table: file_table::FileTable,
}

impl WondFS {
    pub fn new() -> WondFS {
        let mut file = vec![];
        for _ in 0..30 {
            file.push(None);
        }
        let table = file_table::FileTable::new();
        table.inode_manager.core_manager.borrow_mut().mount();
        WondFS {
            file,
            max_file: 16,
            file_table: table,
        }
    }

    pub fn arg_fd(&self, fd: u32) -> Option<file_table::FileLink> {
        let mut file = None;
        if fd < 0 || fd >= self.max_file || self.file[fd as usize].is_none()  {
            return None;
        }
        file = Some(Arc::clone(&self.file[fd as usize].as_ref().unwrap()));
        Some(file.unwrap())
    }
}

impl Filesystem for WondFS {

    // Look up a directory entry by name and get its attributes.
    fn lookup(&mut self, _req: &Request<'_>, _parent: u64, _name: &std::ffi::OsStr, reply: ReplyEntry) {
        let parent = _parent as u32;
        let name = _name.to_str().unwrap().to_string();
        let parent_inode = self.file_table.inode_manager.i_get(parent);
        if parent_inode.is_none() {
            reply.error(ENOENT);
            return;
        }
        let ino = directory::dir_lookup(parent_inode.as_ref().unwrap(), name);
        if ino.is_none() {
            reply.error(ENOENT);
            return;
        }
        let inode = self.file_table.inode_manager.i_get(ino.unwrap().1 as u32);
        if inode.is_none() {
            reply.error(ENOENT);
            return;
        }
        let stat = inode.as_ref().unwrap().borrow().get_stat();
        let attr = transfer_stat_to_attr(stat);
        self.file_table.inode_manager.i_put(parent_inode.unwrap());
        self.file_table.inode_manager.i_put(inode.unwrap());
        reply.entry(&TTL, &attr, 0);
    }

    // Get file attributes.
    fn getattr(&mut self, _req: &Request<'_>, _ino: u64, reply: ReplyAttr) {
        let ino = _ino as u32;
        let inode = self.file_table.inode_manager.i_get(ino);
        match inode {
            Some(inode) => {
                let stat = inode.borrow().get_stat();
                let attr = transfer_stat_to_attr(stat);
                self.file_table.inode_manager.i_put(inode);
                reply.attr(&TTL, &attr);
            },
            None => reply.error(ENOENT),
        }
    }

    // Set file attributes.
    fn setattr(&mut self, _req: &Request<'_>, _ino: u64, _mode: Option<u32>, _uid: Option<u32>, _gid: Option<u32>, _size: Option<u64>, _atime: Option<TimeOrNow>, _mtime: Option<TimeOrNow>, _ctime: Option<std::time::SystemTime>, _fh: Option<u64>, _crtime: Option<std::time::SystemTime>, _chgtime: Option<std::time::SystemTime>, _bkuptime: Option<std::time::SystemTime>, _flags: Option<u32>, reply: ReplyAttr) {

    }

    // Read data.
    fn read(&mut self, _req: &Request<'_>, _ino: u64, _fh: u64, _offset: i64, _size: u32, _flags: i32, _lock_owner: Option<u64>, reply: ReplyData) {
        let ino =  _ino as u32;
        let offset = _offset as u32;
        let size = _size as u32;
        let inode = self.file_table.inode_manager.i_get(ino);
        match inode {
            Some(inode) => {
                let mut data = vec![]; 
                let ret = inode.borrow_mut().read(offset, size, &mut data);
                if ret >= 0 {
                    self.file_table.inode_manager.i_put(inode);
                    reply.data(&data[0..]);
                } else {
                    reply.error(ENOENT);
                }
            },
            None => reply.error(ENOENT),
        }
    }

    // Write data.
    fn write(&mut self, _req: &Request<'_>, _ino: u64, _fh: u64, _offset: i64, _data: &[u8], _write_flags: u32, _flags: i32, _lock_owner: Option<u64>, reply: ReplyWrite) {
        let ino = _ino as u32;
        let offset = _offset as u32;
        let data = _data;
        let inode = self.file_table.inode_manager.i_get(ino);
        match inode {
            Some(inode) => {
                let ret = inode.borrow_mut().write(offset, data.len() as u32, &data.to_vec());
                self.file_table.inode_manager.i_put(inode);
                if ret {
                    reply.written(data.len() as u32);
                } else {
                    reply.error(libc::ENOENT)
                }
            },
            None => reply.error(libc::ENOENT),
        }
    }

    // Read directory.
    fn readdir(&mut self, _req: &Request<'_>, _ino: u64, _fh: u64, _offset: i64, mut reply: ReplyDirectory) {
        let ino = _ino as u32;
        let offset = _offset as i32;
        let inode = self.file_table.inode_manager.i_get(ino);
        if inode.is_none() {
            reply.error(ENOENT);
            return;
        }
        let mut data = vec![];
        inode.as_ref().unwrap().borrow_mut().read_all(&mut data);
        let iter = directory::DirectoryParser::new(&data);
        for (index, entry) in iter.skip(offset as usize).enumerate() {
            let buffer_full: bool = reply.add(
                entry.ino as u64,
                offset as i64 + index as i64 + 1,
                FileType::RegularFile,
                OsStr::from_bytes(entry.file_name.as_bytes()),
            );
            if buffer_full {
                break;
            }
        }
        reply.ok()
    }

    // Release an open directory.
    fn releasedir(&mut self, _req: &Request<'_>, _ino: u64, _fh: u64, _flags: i32, reply: ReplyEmpty) {
        let ino = _ino as u32;
        let inode = self.file_table.inode_manager.i_get(ino);
        if inode.is_none() {
            reply.error(ENOENT);
            return;
        }
        inode.as_ref().unwrap().borrow_mut().ref_cnt -= 1;
        self.file_table.inode_manager.i_put(inode.unwrap());
        reply.ok();
    }

    // Create a file node.
    fn mknod(&mut self, _req: &Request<'_>, _parent: u64, _name: &std::ffi::OsStr, _mode: u32, _umask: u32, _rdev: u32, reply: ReplyEntry) {
        let parent = _parent as u32;
        let name = _name.to_str().unwrap().to_string();
        let parent_inode = self.file_table.inode_manager.i_get(parent);
        if directory::dir_lookup(parent_inode.as_ref().unwrap(), name).is_some() {
            reply.error(libc::ENOENT);
            return;
        }
        let mut inode = self.file_table.inode_manager.i_alloc();
        if inode.is_none() {
            reply.error(libc::ENOENT);
            return;
        }
        let ino = inode.as_ref().unwrap().borrow().ino;
        let mut stat = inode.as_ref().unwrap().borrow().get_stat();
        stat.file_type = inode::InodeFileType::Directory;
        inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        directory::dir_link(inode.as_mut().unwrap(), ino, ".".to_string());
        directory::dir_link(inode.as_mut().unwrap(), parent, "..".to_string());
        let stat = inode.as_ref().unwrap().borrow().get_stat();
        let attr = transfer_stat_to_attr(stat);
        self.file_table.inode_manager.i_put(parent_inode.unwrap());
        self.file_table.inode_manager.i_put(inode.unwrap());
        reply.entry(&TTL, &attr, 0);
    }

    // Create a directory.
    fn mkdir(&mut self, _req: &Request<'_>, _parent: u64, _name: &std::ffi::OsStr, _mode: u32, _umask: u32, reply: ReplyEntry) {
        let parent = _parent as u32;
        let name = _name.to_str().unwrap().to_string();
        let parent_inode = self.file_table.inode_manager.i_get(parent);
        if directory::dir_lookup(parent_inode.as_ref().unwrap(), name).is_some() {
            reply.error(libc::ENOENT);
            return;
        }
        let mut inode = self.file_table.inode_manager.i_alloc();
        if inode.is_none() {
            reply.error(libc::ENOENT);
            return;
        }
        let ino = inode.as_ref().unwrap().borrow().ino;
        let mut stat = inode.as_ref().unwrap().borrow().get_stat();
        stat.file_type = inode::InodeFileType::Directory;
        inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        directory::dir_link(inode.as_mut().unwrap(), ino, ".".to_string());
        directory::dir_link(inode.as_mut().unwrap(), parent, "..".to_string());
        let stat = inode.as_ref().unwrap().borrow().get_stat();
        let attr = transfer_stat_to_attr(stat);
        self.file_table.inode_manager.i_put(parent_inode.unwrap());
        self.file_table.inode_manager.i_put(inode.unwrap());
        reply.entry(&TTL, &attr, 0);
    }

    // Create a hard link.
    fn link(&mut self, _req: &Request<'_>, _ino: u64, _newparent: u64, _newname: &std::ffi::OsStr, reply: ReplyEntry) {
        let ino = _ino as u32;
        let newparent = _newparent as u32;
        let newname = _newname.to_str().unwrap().to_string();
        let mut parent_inode = self.file_table.inode_manager.i_get(newparent);
        if parent_inode.is_none() {
            reply.error(ENOENT);
            return;
        }
        let inode = self.file_table.inode_manager.i_get(ino);
        if inode.is_none() {
            reply.error(ENOENT);
            return;
        }
        let ret = directory::dir_link(parent_inode.as_mut().unwrap(), ino, newname);
        if !ret {
            reply.error(ENOENT);
            return;
        }
        let mut stat = inode.as_ref().unwrap().borrow_mut().get_stat();
        stat.n_link += 1;
        let ret = inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        if !ret {
            reply.error(ENOENT);
            return;
        }
        self.file_table.inode_manager.i_put(parent_inode.unwrap());
        self.file_table.inode_manager.i_put(inode.unwrap());
        let attr = transfer_stat_to_attr(stat);
        reply.entry(&TTL, &attr, 0);
    }

    // Remove a file.
    fn unlink(&mut self, _req: &Request<'_>, _parent: u64, _name: &std::ffi::OsStr, reply: ReplyEmpty) {
        let parent = _parent as u32;
        let name = _name.to_str().unwrap().to_string();
        let mut parent_inode = self.file_table.inode_manager.i_get(parent);
        if parent_inode.is_none() {
            reply.error(ENOENT);
            return;
        }
        let ino = directory::dir_lookup(parent_inode.as_ref().unwrap(), name.clone());
        if ino.is_none() {
            reply.error(ENOENT);
            return;
        }
        let ino = ino.unwrap().1 as u32;
        let inode = self.file_table.inode_manager.i_get(ino);
        if inode.is_none() {
            reply.error(ENOENT);
            return;
        }
        let ret = directory::dir_unlink(parent_inode.as_mut().unwrap(), ino, name);
        if !ret {
            reply.error(ENOENT);
            return;
        }
        let mut stat = inode.as_ref().unwrap().borrow_mut().get_stat();
        stat.n_link -= 1;
        let ret = inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        if !ret {
            reply.error(ENOENT);
            return;
        }
        self.file_table.inode_manager.i_put(parent_inode.unwrap());
        self.file_table.inode_manager.i_put(inode.unwrap());
        reply.ok();
    }

    // Release an open file.
    fn release(&mut self, _req: &Request<'_>, _ino: u64, _fh: u64, _flags: i32, _lock_owner: Option<u64>, _flush: bool, reply: ReplyEmpty) {
        let ino = _ino as u32;
        let inode = self.file_table.inode_manager.i_get(ino);
        if inode.is_none() {
            reply.error(ENOENT);
            return;
        }
        inode.as_ref().unwrap().borrow_mut().ref_cnt -= 1;
        self.file_table.inode_manager.i_put(inode.unwrap());
        reply.ok();
    }

    // Remove a directory.
    fn rmdir(&mut self, _req: &Request<'_>, _parent: u64, _name: &std::ffi::OsStr, reply: ReplyEmpty) {
        let parent = _parent as u32;
        let name = _name.to_str().unwrap().to_string();

    }

    // Rename a file.
    fn rename(&mut self, _req: &Request<'_>, _parent: u64, _name: &OsStr, _newparent: u64, _newname: &OsStr, _flags: u32, reply: ReplyEmpty) {

    }

    // Open a file.
    fn open(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: ReplyOpen) {
        
    }

    // Create and open a file.
    fn create(&mut self, _req: &Request<'_>, _parent: u64, _name: &std::ffi::OsStr, _mode: u32, _umask: u32, _flags: i32, reply: ReplyCreate) {

    }

    // Open a directory.
    fn opendir(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: ReplyOpen) {
        
    }    

}
