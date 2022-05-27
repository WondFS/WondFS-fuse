use std::sync::Mutex;
use std::cmp::{max, min};
use crate::inode::inode_event;

use super::inode_manager;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum InodeFileType {
    File,
    Directory,
}

#[derive(Copy, Clone)]
pub struct InodeStat {
    pub file_type: InodeFileType,
    pub ino: u32,
    pub size: u32,
    pub ref_cnt: u8,
    pub n_link: u8,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub last_accessed: (i64, u32),
    pub last_modified: (i64, u32),
    pub last_metadata_changed: (i64, u32),
}

#[derive(Copy, Clone)]
pub struct InodeEntry {
    pub valid: bool,
    pub offset: u32,
    pub len: u32,           // 以Byte为单位
    pub size: u32,          // 以Page为单位
    pub address: u32,
}

pub struct Inode {
    pub file_type: InodeFileType,
    pub ino: u32,
    pub size: u32,
    pub ref_cnt: u8,
    pub n_link: u8,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub last_accessed: (i64, u32),
    pub last_modified: (i64, u32),
    pub last_metadata_changed: (i64, u32),
    pub lock: Mutex<bool>,
    pub data: Vec<InodeEntry>,
    pub core: Option<inode_manager::CoreLink>,
}

impl Inode {
    pub fn new() -> Inode {
        Inode {
            file_type: InodeFileType::File,
            ino: 0,
            size: 0,
            uid: 0,
            gid: 0,
            n_link: 0,
            data: vec![],
            ref_cnt: 0,
            lock: Mutex::new(false),
            core: None,
            mode: 0,
            last_accessed: (0, 0),
            last_modified: (0, 0),
            last_metadata_changed: (0, 0),
        }
    }

    pub fn read_all(&mut self, buf: &mut Vec<u8>) -> i32 {
        self.read(0, self.size, buf)
    }

    pub fn read(&mut self, offset: u32, len: u32, buf: &mut Vec<u8>) -> i32 {
        buf.clear();
        let mut len = len;
        let mut count = 0;
        let mut flag = false;
        // if self.size == 0 {
        //     return 0;
        // }
        if offset >= self.size {
            return -1;
        }
        if offset + len > self.size {
            len = self.size - offset;
        }
        for entry in self.data.clone().iter() {
            if entry.offset + entry.len < offset {
                continue;
            }
            let cur_count;
            if !flag {
                flag = true;
                cur_count = min(len, entry.offset + entry.len - offset);
            } else {
                cur_count = min(len, entry.len);
            }
            let data = self.read_entry(&entry, 0, cur_count);
            for byte in data.into_iter() {
                buf.push(byte);
            }
            len -= cur_count;
            count += cur_count as i32;
            if len == 0 {
                break;
            }
        }
        count
    }

    pub fn write(&mut self, offset: u32, len: u32, buf: &Vec<u8>) -> bool {
        let mut event_group = inode_event::InodeEventGroup::new();
        event_group.inode = self.copy_inode();
        let mut index = 0;
        let mut flag = false;
        let new_entry = InodeEntry {
            offset,
            len,
            valid: false,
            size: len,
            address: 0,
        };
        let mut second_entry = None;
        let mut second_o_entry = None;
        let mut second_index = 0;
        if offset > self.size {
            return false;
        }
        for entry in self.data.iter() {
            if entry.offset + entry.len <= new_entry.offset {
                index += 1;
                continue
            } else if entry.offset >= new_entry.offset + new_entry.len {
                continue
            } else {
                let valid_prev = max(0, new_entry.offset as i32 - entry.offset as i32) as u32;
                let valid_suffix = max(0, entry.offset as i32 + entry.len as i32 - new_entry.offset as i32 - new_entry.len as i32) as u32;
                if valid_prev == 0 {
                    let event = inode_event::DeleteContentInodeEvent {
                        index,
                        size: entry.size,
                        v_address: entry.address,
                    };
                    event_group.events.push(inode_event::InodeEvent::DeleteContent(event));
                } else {
                    let event = inode_event::TruncateContentInodeEvent {
                        index,
                        offset: entry.offset,
                        len: valid_prev,
                        size: valid_prev / 4096 + 1,
                        o_size: entry.size,
                        v_address: entry.address,
                    };
                    event_group.events.push(inode_event::InodeEvent::TruncateContent(event));
                }
                index += 1;
                if !flag {
                    let event = inode_event::AddContentInodeEvent {
                        index,
                        offset,
                        len,
                        size: len / 4096 + 1,
                        content: buf.clone(),
                    };
                    event_group.events.push(inode_event::InodeEvent::AddContent(event));
                    index += 1;
                    flag = true;
                }
                if valid_suffix > 0 {
                    second_o_entry= Some(entry.clone());
                    second_entry = Some(InodeEntry {
                        offset: entry.offset + entry.len - valid_suffix,
                        len: valid_suffix,
                        valid: false,
                        size: valid_suffix / 4096 + 1,
                        address: 0,
                    });
                    second_index = index;
                }
            }
        }
        if !flag {
            let event = inode_event::AddContentInodeEvent {
                index: self.data.len() as u32,
                offset: new_entry.offset,
                len: new_entry.len,
                size: len / 4096 + 1,
                content: buf.clone(),
            };
            event_group.events.push(inode_event::InodeEvent::AddContent(event));
        }
        if second_entry.is_some() {
            let second_entry = second_entry.unwrap();
            let data = self.read_entry(&second_o_entry.unwrap(), second_entry.offset - second_o_entry.unwrap().offset, second_entry.offset + second_entry.len - second_o_entry.unwrap().offset);
            let event = inode_event::AddContentInodeEvent {
                index: second_index,
                offset: second_entry.offset,
                len: second_entry.len,
                size: second_entry.size,
                content: data,
            };
            event_group.events.push(inode_event::InodeEvent::AddContent(event));
        }
        let inode = self.core.as_mut().unwrap().borrow_mut().dispose_event_group(event_group).unwrap();
        self.update_by_another_inode(inode);
        true
    }

    pub fn insert(&mut self, offset: u32, len: u32, buf: &Vec<u8>) -> bool {
        let mut event_group = inode_event::InodeEventGroup::new();
        event_group.inode = self.copy_inode();
        let mut index = 0;
        let mut flag = false;
        let new_entry = InodeEntry {
            offset,
            len,
            valid: false,
            size: len,
            address: 0,
        };
        let mut second_entry = None;
        let mut second_o_entry = None;
        let mut second_index = 0;
        if offset > self.size {
            return false;
        }
        for entry in self.data.iter_mut() {
            if flag {
                let event = inode_event::ChangeContentInodeEvent {
                    index: index,
                    offset: entry.offset + len,
                    v_address: entry.address,
                };
                event_group.events.push(inode_event::InodeEvent::ChangeContent(event));
            } else {
                if new_entry.offset < entry.offset + entry.len {
                    flag = true;
                    let valid_prev = max(0, new_entry.offset - entry.offset);
                    let valid_suffix = max(0, entry.offset as i32 + entry.len as i32 - new_entry.offset as i32) as u32;
                    if valid_prev == 0 {
                        let event = inode_event::DeleteContentInodeEvent {
                            index,
                            size: entry.size,
                            v_address: entry.address,
                        };
                        event_group.events.push(inode_event::InodeEvent::DeleteContent(event));
                    } else {
                        let event = inode_event::TruncateContentInodeEvent {
                            index,
                            offset: entry.offset,
                            len: valid_prev,
                            size: valid_prev / 4096 + 1,
                            o_size: entry.size,
                            v_address: entry.address,
                        };
                        event_group.events.push(inode_event::InodeEvent::TruncateContent(event));
                    }
                    index  += 1;
                    let event = inode_event::AddContentInodeEvent {
                        index,
                        offset,
                        len,
                        size: len / 4096 + 1,
                        content: buf.clone(),
                    };
                    event_group.events.push(inode_event::InodeEvent::AddContent(event));
                    index += 1;
                    if valid_suffix > 0 {
                        second_o_entry= Some(entry.clone());
                        second_entry = Some(InodeEntry {
                            offset: entry.offset + entry.len + len - valid_suffix,
                            len: valid_suffix,
                            valid: false,
                            size: valid_suffix / 4096 + 1,
                            address: 0,
                        });
                        second_index = index;
                    }
                }
            }
            index += 1;
        }
        if !flag {
            let event = inode_event::AddContentInodeEvent {
                index: self.data.len() as u32,
                offset: new_entry.offset,
                len: new_entry.len,
                size: len / 4096 + 1,
                content: buf.clone(),
            };
            event_group.events.push(inode_event::InodeEvent::AddContent(event));
        }
        if second_entry.is_some() {
            let second_entry = second_entry.unwrap();
            let data = self.read_entry(&second_o_entry.unwrap(), second_entry.offset - len - second_o_entry.unwrap().offset, second_entry.offset + second_entry.len - len - second_o_entry.unwrap().offset);
            let event = inode_event::AddContentInodeEvent {
                index: second_index,
                offset: second_entry.offset,
                len: second_entry.len,
                size: second_entry.size,
                content: data,
            };
            event_group.events.push(inode_event::InodeEvent::AddContent(event));
        }
        let inode = self.core.as_mut().unwrap().borrow_mut().dispose_event_group(event_group).unwrap();
        self.update_by_another_inode(inode);
        true
    }

    pub fn truncate(&mut self, offset: u32, len: u32) -> bool {
        let mut event_group = inode_event::InodeEventGroup::new();
        event_group.inode = self.copy_inode();
        let mut new_entry = None;
        let mut new_o_entry = None;
        let mut new_index = 0;
        let mut index = 0;
        for entry in self.data.iter_mut() {
            if entry.offset + entry.len <= offset {
                index += 1;
                continue
            } else if entry.offset >= offset + len {
                let event = inode_event::ChangeContentInodeEvent {
                    index,
                    offset: entry.offset - len,
                    v_address: entry.address,
                };
                event_group.events.push(inode_event::InodeEvent::ChangeContent(event));
                index += 1;
                continue
            } else {
                let valid_prev = max(0, offset as i32 - entry.offset as i32) as u32;
                let valid_suffix = max(0, entry.offset as i32 + entry.len as i32 - offset as i32 - len as i32) as u32;
                if valid_prev == 0 {
                    let event = inode_event::DeleteContentInodeEvent {
                        index,
                        size: entry.size,
                        v_address: entry.address,
                    };
                    event_group.events.push(inode_event::InodeEvent::DeleteContent(event));
                } else {
                    let event = inode_event::TruncateContentInodeEvent {
                        index: (index as u32),
                        offset: entry.offset,
                        len: valid_prev,
                        size: valid_prev / 4096 + 1,
                        o_size: entry.size,
                        v_address: entry.address,
                    };
                    event_group.events.push(inode_event::InodeEvent::TruncateContent(event));
                }
                if valid_suffix > 0 {
                    new_o_entry = Some(entry.clone());
                    new_entry = Some(InodeEntry {
                        offset: entry.offset + entry.len - valid_suffix - len,
                        len: valid_suffix,
                        valid: false,
                        size: valid_suffix / 4096 + 1,
                        address: 0,
                    });
                    new_index = index;
                    index += 1;
                }
                index += 1;
            }
        }
        if new_entry.is_some() {
            let new_entry = new_entry.unwrap();
            let data = self.read_entry(&new_o_entry.unwrap(), new_entry.offset + len - new_o_entry.unwrap().offset, new_entry.offset + new_entry.len + len - new_o_entry.unwrap().offset);
            let event = inode_event::AddContentInodeEvent {
                index: new_index,
                offset: new_entry.offset,
                len: new_entry.len,
                size: new_entry.size,
                content: data,
            };
            event_group.events.push(inode_event::InodeEvent::AddContent(event));
        }
        let inode = self.core.as_mut().unwrap().borrow_mut().dispose_event_group(event_group).unwrap();
        self.update_by_another_inode(inode);
        true
    }

    pub fn truncate_to_end(&mut self, offset: u32) -> bool {
        self.truncate(offset, self.size - offset)
    }
}

impl Inode {
    pub fn get_stat(&self) -> InodeStat {
        InodeStat {
            file_type: self.file_type,
            ino: self.ino,
            size: self.size,
            uid: self.uid,
            gid: self.gid,
            ref_cnt: self.ref_cnt,
            n_link: self.n_link,
            mode: self.mode,
            last_accessed: self.last_accessed,
            last_modified: self.last_modified,
            last_metadata_changed: self.last_metadata_changed,
        }
    }

    pub fn modify_stat(&mut self, stat: InodeStat) -> bool {
        let mut event_group = inode_event::InodeEventGroup::new();
        event_group.inode = self.copy_inode();
        if stat.ino != self.ino {
            panic!("Inode: modify stat can't change ino");
        }
        if stat.size != self.size {
            panic!("Inode: modify stat can't change size");
        }
        let event = inode_event::ModifyInodeStatInodeEvent {
            file_type: stat.file_type,
            ino: stat.ino,
            size: stat.size,
            uid: stat.uid,
            gid: stat.gid,
            n_link: stat.n_link,
        };
        event_group.events.push(inode_event::InodeEvent::ModifyStat(event));
        let inode = self.core.as_mut().unwrap().borrow_mut().dispose_event_group(event_group).unwrap();
        self.update_by_another_inode(inode);
        true
    }

    pub fn dup(&mut self) -> bool {
        let stat = InodeStat {
            file_type: self.file_type,
            ino: self.ino,
            size: self.size,
            uid: self.uid,
            gid: self.gid,
            ref_cnt: self.ref_cnt,
            n_link: self.n_link + 1,
            mode: self.mode,
            last_accessed: self.last_accessed,
            last_modified: self.last_modified,
            last_metadata_changed: self.last_metadata_changed,
        };
        self.modify_stat(stat)
    }

    pub fn delete(&mut self) -> bool {
        let mut event_group = inode_event::InodeEventGroup::new();
        event_group.inode = self.copy_inode();
        event_group.need_delete = true;
        if self.core.as_mut().unwrap().borrow_mut().dispose_event_group(event_group).is_some() {
            panic!("Inode: delete internal error");
        }
        true
    }
}

impl Inode {
    pub fn read_entry(&mut self, entry: &InodeEntry, start: u32, end: u32) -> Vec<u8> {
        let start_index = start / 4096;
        let start_off = start % 4096;
        let end_index = (end - 1) / 4096;
        let end_off = (end - 1) % 4096;
        let mut pages = vec![];
        for i in start_index..end_index + 1 {
            pages.push(self.core.as_mut().unwrap().borrow_mut().read_data(entry.address + i));
        }
        let mut res = vec![];
        if end_index - start_index > 0 {
            for byte in pages[0][start_off as usize..4096].iter() {
                res.push(*byte);
            }
        } else {
            for byte in pages[0][start_off as usize..(end_off + 1) as usize].iter() {
                res.push(*byte);
            }
        }
        if end_index - start_index > 1 {
            for i in 1..end_index - start_index {
                res.append(&mut pages[i as usize].to_vec());
            }
        }
        if end_index - start_index > 0 {
            res.append(&mut pages[(end_index - start_index) as usize][0..(end_off + 1) as usize].to_vec());
        }
        res
    }

    pub fn update_by_another_inode(&mut self, inode: Inode) {
        self.file_type = inode.file_type;
        self.ino = inode.ino;
        self.size = inode.size;
        self.uid = inode.uid;
        self.gid = inode.gid;
        self.ref_cnt = inode.ref_cnt;
        self.n_link = inode.n_link;
        self.data = inode.data;
    }

    pub fn copy_inode(&self) -> Inode {
        Inode {
            file_type: self.file_type,
            ino: self.ino,
            size: self.size,
            uid: self.uid,
            gid: self.gid,
            n_link: self.n_link,
            data: self.data.clone(),
            ref_cnt: self.ref_cnt,
            lock: Mutex::new(false),
            core: None,
            mode: self.mode,
            last_accessed: self.last_accessed,
            last_modified: self.last_modified,
            last_metadata_changed: self.last_metadata_changed,
        }
    }

    pub fn debug(&self) {
        // println!("Inode::Debug:{}", self.ino);
        // for (index, entry) in self.data.iter().enumerate() {
        //     println!("{} offset: {} len: {} size: {} address: {} valid: {}", index, entry.offset, entry.len, entry.size, entry.address, entry.valid);
        // }
    }
}

#[cfg(test)]
mod test {
    use crate::inode::inode_manager;
    use super::*;

    #[test]
    fn basics() {

    }

    #[test]
    fn write() {
        let mut inode_manager = inode_manager::InodeManager::new();
        inode_manager.core_manager.borrow_mut().mount();
        let mut link = inode_manager.i_alloc();
        let mut buf_1 = vec![];
        for _ in 0..100 {
            buf_1.push(22);
        }
        let mut buf_2 = vec![];
        for _ in 0..27 {
            buf_2.push(31);
        }
        let mut buf_3 = vec![];
        for _ in 0..10 {
            buf_3.push(51);
        }
        let mut buf_4 = vec![];
        for _ in 0..30 {
            buf_4.push(21);
        }
        link.as_ref().unwrap().borrow_mut().write(0, 100, &buf_1);
        link.as_ref().unwrap().borrow_mut().write(13, 27, &buf_2);
        link.as_ref().unwrap().borrow_mut().write(89, 10, &buf_3);
        link.as_mut().unwrap().borrow_mut().write(5, 30, &buf_4);
        let mut buf = vec![];
        link.as_mut().unwrap().borrow_mut().read_all(&mut buf);
        assert_eq!(buf.len(), 100);
        link.as_mut().unwrap().borrow_mut().read(10, 80, &mut buf);
        assert_eq!(buf.len(), 80);
        let mut buf_5 = vec![];
        for _ in 0..10000 {
            buf_5.push(37)
        }
        link.as_mut().unwrap().borrow_mut().write(5, 10000, &buf_5);
        link.as_mut().unwrap().borrow_mut().read_all(&mut buf);
        assert_eq!(buf.len(), 10005);
        link.as_mut().unwrap().borrow_mut().read(50, 8000, &mut buf);
        assert_eq!(buf.len(), 8000);
    }

    #[test]
    fn insert() {
        let mut inode_manager = inode_manager::InodeManager::new();
        inode_manager.core_manager.borrow_mut().mount();
        let mut link = inode_manager.i_alloc();
        let mut buf_1 = vec![];
        for _ in 0..100 {
            buf_1.push(22);
        }
        let mut buf_2 = vec![];
        for _ in 0..30 {
            buf_2.push(31);
        }
        let mut buf_3 = vec![];
        for _ in 0..10 {
            buf_3.push(51);
        }
        let mut buf_4 = vec![];
        for _ in 0..30 {
            buf_4.push(21);
        }
        link.as_ref().unwrap().borrow_mut().insert(0, 100, &buf_1);
        link.as_ref().unwrap().borrow_mut().insert(40, 30, &buf_2);
        link.as_ref().unwrap().borrow_mut().insert(45, 10, &buf_3);
        link.as_mut().unwrap().borrow_mut().insert(35, 30, &buf_4);
        let mut buf = vec![];
        link.as_mut().unwrap().borrow_mut().read_all(&mut buf);
        assert_eq!(buf.len(), 170);
        link.as_mut().unwrap().borrow_mut().read(10, 80, &mut buf);
        assert_eq!(buf.len(), 80);
    }

    #[test]
    fn truncate() {
        let mut inode_manager = inode_manager::InodeManager::new();
        inode_manager.core_manager.borrow_mut().mount();
        let mut link = inode_manager.i_alloc();
        let mut buf_1 = vec![];
        for _ in 0..100 {
            buf_1.push(22);
        }
        let mut buf_2 = vec![];
        for _ in 0..30 {
            buf_2.push(31);
        }
        let mut buf_3 = vec![];
        for _ in 0..10 {
            buf_3.push(51);
        }
        let mut buf_4 = vec![];
        for _ in 0..30 {
            buf_4.push(21);
        }
        link.as_ref().unwrap().borrow_mut().insert(0, 100, &buf_1);
        link.as_ref().unwrap().borrow_mut().insert(40, 30, &buf_2);
        link.as_ref().unwrap().borrow_mut().insert(45, 10, &buf_3);
        link.as_ref().unwrap().borrow_mut().truncate(30, 100);
        let mut buf = vec![];
        link.as_mut().unwrap().borrow_mut().read_all(&mut buf);
        assert_eq!(buf.len(), 40);
    }

    #[test]
    fn modify() {
        let mut inode_manager = inode_manager::InodeManager::new();
        inode_manager.core_manager.borrow_mut().mount();
        let link = inode_manager.i_alloc();
        let stat = InodeStat {
            file_type: InodeFileType::Directory,
            ino: link.as_ref().unwrap().borrow().ino,
            size: 0,
            uid: 100,
            gid: 44,
            ref_cnt: 10,
            n_link: 10,
            mode: 0,
            last_accessed: (0, 0),
            last_modified: (0, 0),
            last_metadata_changed: (0, 0),
        };
        link.as_ref().unwrap().borrow_mut().modify_stat(stat);
        let link = link.as_ref().unwrap().borrow_mut().core.as_mut().unwrap().borrow_mut().get_inode(1);
        assert_eq!(link.uid, 100);
        assert_eq!(link.gid, 44);
    }

    #[test]
    fn delete() {
        let mut inode_manager = inode_manager::InodeManager::new();
        inode_manager.core_manager.borrow_mut().mount();
        let link = inode_manager.i_alloc();
        let mut buf_1 = vec![];
        for _ in 0..100 {
            buf_1.push(22);
        }
        let mut buf_2 = vec![];
        for _ in 0..27 {
            buf_2.push(31);
        }
        link.as_ref().unwrap().borrow_mut().write(0, 100, &buf_1);
        link.as_ref().unwrap().borrow_mut().write(13, 27, &buf_2);
        link.as_ref().unwrap().borrow_mut().delete();
    }
}