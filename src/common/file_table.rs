use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use crate::common::file;
use crate::common::file::FileDescriptorType;
use crate::inode::inode_manager;

pub struct FileTable {
    pub lock: Mutex<bool>,
    pub file: Vec<FileLink>,
    pub max_num: u32,
    pub inode_manager: inode_manager::InodeManager,
}

pub type FileLink = Arc<RefCell<file::File>>;

impl FileTable {
    pub fn new() -> FileTable {
        let mut file = vec![];
        for _ in 0..30 {
            file.push(Arc::new(RefCell::new(file::File::new())));
        }
        FileTable {
            file,
            lock: Mutex::new(false),
            max_num: 100,
            inode_manager: inode_manager::InodeManager::new(),
        }
    }

    // Allocate a file structure.
    pub fn file_alloc(&mut self) -> Option<FileLink> {
        let _ = self.lock.lock();
        for f in self.file.iter() {
            if f.borrow().ref_cnt == 0 {
                f.borrow_mut().ref_cnt += 1;
                return Some(Arc::clone(&f));
            }
        }
        None
    }

    // Increment ref count for file f.
    pub fn file_dup(&mut self, link: &FileLink) -> FileLink {
        let _ = self.lock.lock();
        link.borrow_mut().ref_cnt += 1;
        Arc::clone(link)
    }
    
    // Close file f. (Decrement ref count, close when reaches 0.).
    pub fn file_close(&mut self, link: FileLink) {
        let _ = self.lock.lock();
        if link.borrow().ref_cnt <= 0 {
            panic!("FileTable: close internal error");
        }
        link.borrow_mut().ref_cnt -= 1;
        if link.borrow().ref_cnt == 0 {
            link.borrow_mut().fd_type = FileDescriptorType::NONE;
            self.inode_manager.i_put(link.borrow_mut().inode.take().unwrap());
        }
    }

}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basics() {
        let mut table = FileTable::new();
        table.inode_manager.core_manager.borrow_mut().mount();
        let link = table.file_alloc().unwrap();
        let _ = table.file_dup(&link);
        table.file_close(link);
    }
}