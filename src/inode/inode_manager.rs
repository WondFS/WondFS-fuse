//
// Inode Manager
//

use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use crate::core::core_manager;
use crate::inode::inode::Inode;

pub type CoreLink = Arc<RefCell<core_manager::CoreManager>>;
pub type InodeLink = Arc<RefCell<Inode>>;

// Inode Manager Structure
pub struct InodeManager {
    size: usize,
    capacity: usize,
    inode_buffer: Vec<InodeLink>,
    lock: Mutex<bool>,
    pub core_manager: CoreLink,
}

// Inode Manager Simple Interface Function
impl InodeManager {
    pub fn new() -> InodeManager {
        let mut buf = vec![];
        for _ in 0..30 {
            buf.push(Arc::new(RefCell::new(Inode::new())));
        }
        let capacity = 30;
        InodeManager {
            size: 0,
            capacity: capacity as usize,
            core_manager: Arc::new(RefCell::new(core_manager::CoreManager::new())),
            inode_buffer: buf,
            lock: Mutex::new(false),
        }
    }

    pub fn get_size(&self) -> u32 {
        self.size as u32
    }

    pub fn get_capacity(&self) -> u32 {
        self.capacity as u32
    }
}

// Inode Manger Main Interface Function
impl InodeManager {
    /// Allocate an inode on device dev.
    /// Mark it as allocated by giving it type type.
    /// Returns an unlocked but allocated and referenced inode.
    pub fn i_alloc(&mut self) -> Option<InodeLink> {
        let mut empty_index = -1;
        let _ = self.lock.lock();
        for (index, ip) in self.inode_buffer.iter().enumerate() {
            if empty_index == -1 && ip.borrow().ref_cnt == 0 {
                empty_index = index as i32;
            }
        }
        if empty_index == -1 {
            panic!("InodeManager: alloc no spare cache to store");
        }
        let mut inode = self.core_manager.borrow_mut().allocate_inode();
        inode.ref_cnt = 1;
        let link = Arc::new(RefCell::new(inode));
        link.borrow_mut().core = Some(Arc::clone(&self.core_manager));
        self.inode_buffer[empty_index as usize] = Arc::clone(&link);
        Some(link)
    }

    /// Find the inode with number ino on device dev
    /// and return the in-memory copy.
    pub fn i_get(&mut self, ino: u32) -> Option<InodeLink> {
        let mut empty_index = -1;
        let _ = self.lock.lock();
        for (index, ip) in self.inode_buffer.iter().enumerate() {
            if ip.borrow().ref_cnt > 0 && ip.borrow().ino == ino {
                ip.borrow_mut().ref_cnt += 1;
                return Some(Arc::clone(ip));
            }
            if empty_index == -1 && ip.borrow().ref_cnt == 0 {
                empty_index = index as i32;
            }
        }
        if empty_index == -1 {
            panic!("InodeManager: get no spare cache to store");
        }
        let mut inode = self.core_manager.borrow_mut().get_inode(ino);
        inode.ref_cnt = 1;
        let link = Arc::new(RefCell::new(inode));
        link.borrow_mut().core = Some(Arc::clone(&self.core_manager));
        self.inode_buffer[empty_index as usize] = Arc::clone(&link);
        Some(link)
    }

    /// Increment reference count for ip.
    pub fn i_dup(&mut self, inode: &InodeLink) -> InodeLink {
        let _ = self.lock.lock();
        inode.borrow_mut().ref_cnt += 1;
        Arc::clone(inode)
    }

    /// Drop a reference to an in-memory inode.
    /// If that was the last reference, the inode cache entry can
    /// be recycled.
    pub fn i_put(&mut self, inode: InodeLink) {
        let _ = self.lock.lock();
        if inode.borrow().ref_cnt > 0 {
            inode.borrow_mut().ref_cnt -= 1;
        }
    }
}

// Inode Manager Module Test
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basics() {
        let mut manager = InodeManager::new();
        manager.core_manager.borrow_mut().mount();
        let link = manager.i_alloc();
        assert_eq!(link.unwrap().borrow().ino, 1);
        let link = manager.i_alloc();
        assert_eq!(link.unwrap().borrow().ino, 2);
        let link = manager.i_get(2);
        assert_eq!(link.as_ref().unwrap().borrow().ino, 2);
        assert_eq!(link.as_ref().unwrap().borrow().ref_cnt, 2);
        let link = manager.i_dup(link.as_ref().unwrap());
        assert_eq!(link.as_ref().borrow().ref_cnt, 3);
        manager.i_put(link);
        let link = manager.i_get(2);
        assert_eq!(link.as_ref().unwrap().borrow().ref_cnt, 3);
    }
}