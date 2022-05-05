extern crate fuser;
extern crate libc;

use fuser::*;
use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;
use std::time::Duration;
use std::sync::atomic::{AtomicU64, Ordering};
use libc::{ENOENT, ENOSYS};
use crate::inode::{inode, inode_manager};
use crate::common::{path, directory};
use crate::fuse::fuse_helper::*;

const TTL: Duration = Duration::from_secs(1); // 1 second

pub struct WondFS {
    pub inode_manager: inode_manager::InodeManager,
    pub next_file_handle: AtomicU64,
}

impl WondFS {
    pub fn new() -> WondFS {
        let manager = inode_manager::InodeManager::new();
        manager.core_manager.borrow_mut().mount();
        WondFS {
            inode_manager: manager,
            next_file_handle: AtomicU64::new(1),
        }
    }

    fn allocate_next_file_handle(&self, read: bool, write: bool) -> u64 {
        let mut fh = self.next_file_handle.fetch_add(1, Ordering::SeqCst);
        assert!(fh < FILE_HANDLE_WRITE_BIT && fh < FILE_HANDLE_READ_BIT);
        if read {
            fh |= FILE_HANDLE_READ_BIT;
        }
        if write {
            fh |= FILE_HANDLE_WRITE_BIT;
        }
        fh
    }
}

impl Filesystem for WondFS {
    fn init(&mut self, _req: &Request<'_>, _config: &mut KernelConfig) -> Result<(), libc::c_int> {
        trace!("WondFS: init funcation called");
        Ok(())
    }

    // Look up a directory entry by name and get its attributes.
    fn lookup(&mut self, _req: &Request<'_>, _parent: u64, _name: &std::ffi::OsStr, reply: ReplyEntry) {
        trace!("WondFS: lookup funcation called");
        let parent = _parent as u32;
        let name = _name.to_str().unwrap().to_string();
        trace!("WondFS: parent: {}, name: {}", parent, name);
        let parent_inode = self.inode_manager.i_get(parent);
        if parent_inode.is_none() {
            debug!("WondFS: lookup parent not exists");
            reply.error(ENOENT);
            return;
        }
        if !check_access(
            parent_inode.as_ref().unwrap().borrow().uid,
            parent_inode.as_ref().unwrap().borrow().gid,
            parent_inode.as_ref().unwrap().borrow().mode,
            _req.uid(),
            _req.gid(),
            libc::X_OK,
        ) {
            debug!("WondFS: lookup no permission to access");
            reply.error(libc::ENOENT);
            return;
        }
        let ino = directory::dir_lookup(parent_inode.as_ref().unwrap(), name);
        if ino.is_none() {
            debug!("WondFS: lookup name not exists");
            reply.error(ENOENT);
            return;
        }
        let inode = self.inode_manager.i_get(ino.unwrap().1 as u32);
        if inode.is_none() {
            debug!("WondFS: lookup inode not exists");
            reply.error(ENOENT);
            return;
        }
        let stat = inode.as_ref().unwrap().borrow().get_stat();
        let attr = transfer_stat_to_attr(stat);
        self.inode_manager.i_put(parent_inode.unwrap());
        self.inode_manager.i_put(inode.unwrap());
        reply.entry(&TTL, &attr, 0);
    }

    // Get file attributes.
    fn getattr(&mut self, _req: &Request<'_>, _ino: u64, reply: ReplyAttr) {
        trace!("WondFS: getattr funcation called");
        let ino = _ino as u32;
        trace!("WondFS: ino: {}", ino);
        let inode = self.inode_manager.i_get(ino);
        match inode {
            Some(inode) => {
                let stat = inode.borrow().get_stat();
                let attr = transfer_stat_to_attr(stat);
                self.inode_manager.i_put(inode);
                reply.attr(&TTL, &attr);
            },
            None => {
                debug!("WondFS: getattr ino not exists");
                reply.error(ENOENT);
            },
        }
    }

    // Set file attributes.
    fn setattr(&mut self, _req: &Request<'_>, _ino: u64, _mode: Option<u32>, _uid: Option<u32>, _gid: Option<u32>, _size: Option<u64>, _atime: Option<TimeOrNow>, _mtime: Option<TimeOrNow>, _ctime: Option<std::time::SystemTime>, _fh: Option<u64>, _crtime: Option<std::time::SystemTime>, _chgtime: Option<std::time::SystemTime>, _bkuptime: Option<std::time::SystemTime>, _flags: Option<u32>, reply: ReplyAttr) {
        reply.error(ENOSYS);
    }

    // Create a file node.
    fn mknod(&mut self, _req: &Request<'_>, _parent: u64, _name: &std::ffi::OsStr, mut _mode: u32, _umask: u32, _rdev: u32, reply: ReplyEntry) {
        trace!("WondFS: mknod funcation called");
        let file_type = _mode & libc::S_IFMT as u32;
        if file_type != libc::S_IFREG as u32 && file_type != libc::S_IFDIR as u32 {
            debug!("WondFS: mknod implementation is incomplete. Only supports regular files, and directories. Got {:o}", _mode);
            reply.error(libc::ENOSYS);
            return;
        }
        let parent = _parent as u32;
        let name = _name.to_str().unwrap().to_string();
        trace!("WondFS: parent: {}, name: {}", parent, name);
        let mut parent_inode = self.inode_manager.i_get(parent);
        if parent_inode.is_none() {
            reply.error(ENOENT);
            debug!("WondFS: mknod parent not exists");
            return;
        }
        if directory::dir_lookup(parent_inode.as_ref().unwrap(), name.clone()).is_some() {
            reply.error(libc::ENOENT);
            debug!("WondFS: mknod name has exist");
            return;
        }
        if !check_access(
            parent_inode.as_ref().unwrap().borrow().uid,
            parent_inode.as_ref().unwrap().borrow().gid,
            parent_inode.as_ref().unwrap().borrow().mode,
            _req.uid(),
            _req.gid(),
            libc::X_OK,
        ) {
            debug!("WondFS: mknod no permission to access");
            reply.error(libc::ENOENT);
            return;
        }
        let mut stat = parent_inode.as_ref().unwrap().borrow().get_stat();
        stat.last_modified = time_now();
        stat.last_metadata_changed = time_now();
        parent_inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        if _req.uid() != 0 {
            _mode &= !(libc::S_ISUID | libc::S_ISGID) as u32;
        }
        let mut inode = self.inode_manager.i_alloc();
        if inode.is_none() {
            debug!("WondFS: mknod alloc inode error");
            reply.error(libc::ENOENT);
            return;
        }
        let ino = inode.as_ref().unwrap().borrow().ino;
        let mut stat = inode.as_ref().unwrap().borrow().get_stat();
        let parent_stat = parent_inode.as_ref().unwrap().borrow().get_stat();
        stat.file_type = as_file_kind(_mode);
        stat.size = 0;
        stat.ref_cnt = 0;
        stat.n_link = 1;
        stat.mode = creation_mode(_mode);
        stat.uid = _req.uid();
        stat.gid = creation_gid(&parent_stat, _req.gid());
        stat.last_accessed = time_now();
        stat.last_modified = time_now();
        stat.last_metadata_changed = time_now();
        inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        if inode.as_ref().unwrap().borrow().file_type == inode::InodeFileType::Directory {
            directory::dir_link(inode.as_mut().unwrap(), ino, ".".to_string());
            directory::dir_link(inode.as_mut().unwrap(), parent, "..".to_string());
        }
        directory::dir_link(parent_inode.as_mut().unwrap(), ino, name);
        let attr = transfer_stat_to_attr(stat);
        self.inode_manager.i_put(parent_inode.unwrap());
        self.inode_manager.i_put(inode.unwrap());
        reply.entry(&TTL, &attr, 0);
    }

    // Create a directory.
    fn mkdir(&mut self, _req: &Request<'_>, _parent: u64, _name: &OsStr, mut _mode: u32, _umask: u32, reply: ReplyEntry) {
        trace!("WondFS: mkdir funcation called");
        let parent = _parent as u32;
        let name = _name.to_str().unwrap().to_string();
        trace!("WondFS: parent: {}, name: {}", parent, name);
        let mut parent_inode = self.inode_manager.i_get(parent);
        if parent_inode.is_none() {
            reply.error(ENOENT);
            debug!("WondFS: mkdir parent not exists");
            return;
        }
        if directory::dir_lookup(parent_inode.as_ref().unwrap(), name.clone()).is_some() {
            reply.error(libc::ENOENT);
            debug!("WondFS: mkdir name has exist");
            return;
        }
        if !check_access(
            parent_inode.as_ref().unwrap().borrow().uid,
            parent_inode.as_ref().unwrap().borrow().gid,
            parent_inode.as_ref().unwrap().borrow().mode,
            _req.uid(),
            _req.gid(),
            libc::X_OK,
        ) {
            debug!("WondFS: mkdir no permission to access");
            reply.error(libc::ENOENT);
            return;
        }
        let mut stat = parent_inode.as_ref().unwrap().borrow().get_stat();
        stat.last_modified = time_now();
        stat.last_metadata_changed = time_now();
        parent_inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        if _req.uid() != 0 {
            _mode &= !(libc::S_ISUID | libc::S_ISGID) as u32;
        }
        if parent_inode.as_ref().unwrap().borrow().mode & libc::S_ISGID as u16 != 0 {
            _mode |= libc::S_ISGID as u32;
        }
        let mut inode = self.inode_manager.i_alloc();
        if inode.is_none() {
            debug!("WondFS: mknod alloc inode error");
            reply.error(libc::ENOENT);
            return;
        }
        let ino = inode.as_ref().unwrap().borrow().ino;
        let mut stat = inode.as_ref().unwrap().borrow().get_stat();
        let parent_stat = parent_inode.as_ref().unwrap().borrow().get_stat();
        stat.file_type = inode::InodeFileType::Directory;
        stat.size = 0;
        stat.ref_cnt = 0;
        stat.n_link = 2;
        stat.mode = creation_mode(_mode);
        stat.uid = _req.uid();
        stat.gid = creation_gid(&parent_stat, _req.gid());
        stat.last_accessed = time_now();
        stat.last_modified = time_now();
        stat.last_metadata_changed = time_now();
        inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        directory::dir_link(inode.as_mut().unwrap(), ino, ".".to_string());
        directory::dir_link(inode.as_mut().unwrap(), parent, "..".to_string());
        directory::dir_link(parent_inode.as_mut().unwrap(), ino, name);
        let attr = transfer_stat_to_attr(stat);
        self.inode_manager.i_put(parent_inode.unwrap());
        self.inode_manager.i_put(inode.unwrap());
        reply.entry(&TTL, &attr, 0);
    }

    // Remove a file.
    fn unlink(&mut self, _req: &Request<'_>, _parent: u64, _name: &std::ffi::OsStr, reply: ReplyEmpty) {
        trace!("WondFS: unlink funcation called");
        let parent = _parent as u32;
        let name = _name.to_str().unwrap().to_string();
        trace!("WondFS: parent: {}, name: {}", parent, name);
        let mut parent_inode = self.inode_manager.i_get(parent);
        if parent_inode.is_none() {
            reply.error(ENOENT);
            debug!("WondFS: unlink parent not exists");
            return;
        }
        let ino = directory::dir_lookup(parent_inode.as_ref().unwrap(), name.clone());
        if ino.is_none() {
            reply.error(libc::ENOENT);
            debug!("WondFS: unlink name not exists");
            return;
        }
        let ino = ino.unwrap().1 as u32;
        let inode = self.inode_manager.i_get(ino);
        if inode.is_none() {
            debug!("WondFS: unlink inode not exists");
            reply.error(ENOENT);
            return;
        }
        if !check_access(
            parent_inode.as_ref().unwrap().borrow().uid,
            parent_inode.as_ref().unwrap().borrow().gid,
            parent_inode.as_ref().unwrap().borrow().mode,
            _req.uid(),
            _req.gid(),
            libc::X_OK,
        ) {
            debug!("WondFS: unlink no permission to access");
            reply.error(libc::ENOENT);
            return;
        }
        let uid = _req.uid();
        if parent_inode.as_ref().unwrap().borrow().mode & libc::S_ISVTX as u16 != 0
            && uid != 0
            && uid != parent_inode.as_ref().unwrap().borrow().uid
            && uid != inode.as_ref().unwrap().borrow().uid
        {
            // "Sticky bit" handling
            debug!("WondFS: unlink no permission to access");
            reply.error(ENOENT);
            return;
        }
        let mut stat = parent_inode.as_ref().unwrap().borrow().get_stat();
        stat.last_modified = time_now();
        stat.last_metadata_changed = time_now();
        parent_inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        let ret = directory::dir_unlink(parent_inode.as_mut().unwrap(), ino, name);
        if !ret {
            debug!("WondFS: unlink not success");
            reply.error(ENOENT);
            return;
        }
        let mut stat = inode.as_ref().unwrap().borrow_mut().get_stat();
        stat.n_link -= 1;
        stat.last_metadata_changed = time_now();
        inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        if inode.as_ref().unwrap().borrow().n_link == 0 {
            inode.as_ref().unwrap().borrow_mut().delete();
        }
        self.inode_manager.i_put(parent_inode.unwrap());
        self.inode_manager.i_put(inode.unwrap());
        reply.ok();
    }

    // Remove a directory.
    fn rmdir(&mut self, _req: &Request<'_>, _parent: u64, _name: &std::ffi::OsStr, reply: ReplyEmpty) {
        trace!("WondFS: rmdir funcation called");
        let parent = _parent as u32;
        let name = _name.to_str().unwrap().to_string();
        trace!("WondFS: parent: {}, name: {}", parent, name);
        let mut parent_inode = self.inode_manager.i_get(parent);
        if parent_inode.is_none() {
            reply.error(ENOENT);
            debug!("WondFS: rmdir parent not exists");
            return;
        }
        let ino = directory::dir_lookup(parent_inode.as_ref().unwrap(), name.clone());
        if ino.is_none() {
            reply.error(libc::ENOENT);
            debug!("WondFS: rmdir name not exists");
            return;
        }
        let ino = ino.unwrap().1 as u32;
        let inode = self.inode_manager.i_get(ino);
        if inode.is_none() {
            debug!("WondFS: rmdir inode not exists");
            reply.error(ENOENT);
            return;
        }
        if inode.as_ref().unwrap().borrow().size > 28 {
            debug!("WondFS: rmdir dir not empty");
            reply.error(ENOENT);
            return;
        }
        if !check_access(
            parent_inode.as_ref().unwrap().borrow().uid,
            parent_inode.as_ref().unwrap().borrow().gid,
            parent_inode.as_ref().unwrap().borrow().mode,
            _req.uid(),
            _req.gid(),
            libc::X_OK,
        ) {
            debug!("WondFS: unlink no permission to access");
            reply.error(libc::ENOENT);
            return;
        }
        let uid = _req.uid();
        if parent_inode.as_ref().unwrap().borrow().mode & libc::S_ISVTX as u16 != 0
            && uid != 0
            && uid != parent_inode.as_ref().unwrap().borrow().uid
            && uid != inode.as_ref().unwrap().borrow().uid
        {
            // "Sticky bit" handling
            debug!("WondFS: unlink no permission to access");
            reply.error(ENOENT);
            return;
        }
        let mut stat = parent_inode.as_ref().unwrap().borrow().get_stat();
        stat.last_modified = time_now();
        stat.last_metadata_changed = time_now();
        parent_inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        let ret = directory::dir_unlink(parent_inode.as_mut().unwrap(), ino, name);
        if !ret {
            debug!("WondFS: unlink not success");
            reply.error(ENOENT);
            return;
        }
        let mut stat = inode.as_ref().unwrap().borrow_mut().get_stat();
        stat.n_link = 0;
        stat.last_metadata_changed = time_now();
        inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        inode.as_ref().unwrap().borrow_mut().delete();
        self.inode_manager.i_put(parent_inode.unwrap());
        self.inode_manager.i_put(inode.unwrap());
        reply.ok();
    }
    
    // Rename a file.
    fn rename(&mut self, _req: &Request<'_>, _parent: u64, _name: &OsStr, _newparent: u64, _newname: &OsStr, _flags: u32, reply: ReplyEmpty) {
        reply.error(ENOSYS);
    }

    // Create a hard link.
    fn link(&mut self, _req: &Request<'_>, _ino: u64, _newparent: u64, _newname: &std::ffi::OsStr, reply: ReplyEntry) {
        trace!("WondFS: link funcation called");
        let ino = _ino as u32;
        let newparent = _newparent as u32;
        let newname = _newname.to_str().unwrap().to_string();
        trace!("WondFS: ino: {}, newparent: {}, newname: {}", ino, newparent, newname);
        let mut parent_inode = self.inode_manager.i_get(newparent);
        if parent_inode.is_none() {
            reply.error(ENOENT);
            debug!("WondFS: link parent not exists");
            return;
        }
        let inode = self.inode_manager.i_get(ino);
        if inode.is_none() {
            reply.error(ENOENT);
            debug!("WondFS: link inode not exists");
            return;
        }
        let ret = directory::dir_link(parent_inode.as_mut().unwrap(), ino, newname);
        if !ret {
            debug!("WondFS: link not success");
            reply.error(ENOENT);
            return;
        }
        let mut stat = inode.as_ref().unwrap().borrow_mut().get_stat();
        stat.n_link += 1;
        stat.last_metadata_changed = time_now();
        inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        self.inode_manager.i_put(parent_inode.unwrap());
        self.inode_manager.i_put(inode.unwrap());
        let attr = transfer_stat_to_attr(stat);
        reply.entry(&TTL, &attr, 0);
    }

    // Open a file.
    fn open(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: ReplyOpen) {
        trace!("WondFS: link funcation called");
        let ino = _ino as u32;
        trace!("WondFS: ino: {}", ino);
        let (access_mask, read, write) = match _flags & libc::O_ACCMODE {
            libc::O_RDONLY => {
                if _flags & libc::O_TRUNC != 0 {
                    reply.error(libc::EACCES);
                    return;
                }
                (libc::R_OK, true, false)
            }
            libc::O_WRONLY => (libc::W_OK, false, true),
            libc::O_RDWR => (libc::R_OK | libc::W_OK, true, true),
            _ => {
                reply.error(libc::EINVAL);
                return;
            }
        };
    }

    // Read data.
    fn read(&mut self, _req: &Request<'_>, _ino: u64, _fh: u64, _offset: i64, _size: u32, _flags: i32, _lock_owner: Option<u64>, reply: ReplyData) {
        let ino =  _ino as u32;
        let offset = _offset as u32;
        let size = _size as u32;
        let inode = self.inode_manager.i_get(ino);
        match inode {
            Some(inode) => {
                let mut data = vec![]; 
                let ret = inode.borrow_mut().read(offset, size, &mut data);
                if ret >= 0 {
                    self.inode_manager.i_put(inode);
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
        let inode = self.inode_manager.i_get(ino);
        match inode {
            Some(inode) => {
                let ret = inode.borrow_mut().write(offset, data.len() as u32, &data.to_vec());
                self.inode_manager.i_put(inode);
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
        let inode = self.inode_manager.i_get(ino);
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
        let inode = self.inode_manager.i_get(ino);
        if inode.is_none() {
            reply.error(ENOENT);
            return;
        }
        inode.as_ref().unwrap().borrow_mut().ref_cnt -= 1;
        self.inode_manager.i_put(inode.unwrap());
        reply.ok();
    }

    // Release an open file.
    fn release(&mut self, _req: &Request<'_>, _ino: u64, _fh: u64, _flags: i32, _lock_owner: Option<u64>, _flush: bool, reply: ReplyEmpty) {
        let ino = _ino as u32;
        let inode = self.inode_manager.i_get(ino);
        if inode.is_none() {
            reply.error(ENOENT);
            return;
        }
        inode.as_ref().unwrap().borrow_mut().ref_cnt -= 1;
        self.inode_manager.i_put(inode.unwrap());
        reply.ok();
    }

    // Create and open a file.
    fn create(&mut self, _req: &Request<'_>, _parent: u64, _name: &std::ffi::OsStr, _mode: u32, _umask: u32, _flags: i32, reply: ReplyCreate) {
        let parent = _parent as u32;
        let name = _name.to_str().unwrap().to_string();
        let parent_inode = self.inode_manager.i_get(parent);
        if parent_inode.is_none() {
            reply.error(ENOENT);
            return;
        }
        let ino = directory::dir_lookup(parent_inode.as_ref().unwrap(), name.clone());
        if ino.is_some() {
            reply.error(ENOENT);
            return;
        }
        let mut stat = parent_inode.as_ref().unwrap().borrow().get_stat();
        stat.last_modified = time_now();
        stat.last_metadata_changed = time_now();
        parent_inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        let mut inode = self.inode_manager.i_alloc();
        if inode.is_none() {
            reply.error(ENOENT);
            return;
        }
        let ino = inode.as_ref().unwrap().borrow().ino;
        let mut stat = parent_inode.as_ref().unwrap().borrow().get_stat();
        stat.file_type = inode::InodeFileType::File;
        stat.n_link = 1;
        stat.ref_cnt = 1;
        stat.size = 0;
        stat.last_modified = time_now();
        stat.last_metadata_changed = time_now();
        inode.as_ref().unwrap().borrow_mut().modify_stat(stat);
        if inode.as_ref().unwrap().borrow().file_type == inode::InodeFileType::Directory {
            directory::dir_link(inode.as_mut().unwrap(), ino, ".".to_string());
            directory::dir_link(inode.as_mut().unwrap(), parent, "..".to_string());
        }
        self.inode_manager.i_put(parent_inode.unwrap());
        let stat = inode.as_ref().unwrap().borrow().get_stat();
        let attr = transfer_stat_to_attr(stat);
        // reply.created(&TTL, &attr, 0, fh, 0);
    }

    // Open a directory.
    fn opendir(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: ReplyOpen) {
        
    }    

}