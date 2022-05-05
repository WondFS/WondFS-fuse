use crate::inode::inode;
use crate::inode::inode_manager;

// Look for a directory entry in a directory
pub fn dir_lookup(inode: &inode_manager::InodeLink, name: String) -> Option<(u32, usize)> {
    if inode.borrow().file_type != inode::InodeFileType::Directory {
        return None;
    }
    let mut buf = vec![];
    if inode.borrow_mut().read_all(&mut buf) == 0 {
        return None;
    }
    let iter = DirectoryParser::new(&buf);
    for (i, entry) in iter.enumerate() {
        if entry.ino == 0 {
            continue;
        }
        if entry.file_name == name {
            return Some((entry.ino, i));
        }
    }
    None
}

// Write a new directory entry (name, ino) into the directory inode.
pub fn dir_link(inode: &mut inode_manager::InodeLink, ino: u32, name: String) -> bool {
    if dir_lookup(&inode, name.clone()).is_some() {
        return false;
    }
    let mut buf = vec![];
    if inode.borrow_mut().read_all(&mut buf) == 0 {
        return false;
    }
    let iter = DirectoryParser::new(&buf);
    let mut index = 0;
    let per_size = iter.per_size;
    for entry in iter {
        if entry.ino == 0 {
            break;
        }
        index += 1;
    }
    let entry = DirectoryInodeEntry {
        file_name: name,
        ino,
    };
    let buf = DirectoryParser::encode(&entry).unwrap();
    inode.borrow_mut().write((index * per_size) as u32, per_size as u32, &buf)
}

// Delete a directory entry (name, ino) into the directory inode.
pub fn dir_unlink(inode: &mut inode_manager::InodeLink, ino: u32, name: String) -> bool {
    if !dir_lookup(&inode, name.clone()).is_some() {
        return false;
    }
    let mut buf = vec![];
    if inode.borrow_mut().read_all(&mut buf) == 0 {
        return false;
    }
    let iter = DirectoryParser::new(&buf);
    let mut index = 0;
    let per_size = iter.per_size;
    let len = iter.len;
    for entry in iter {
        if entry.ino == ino && entry.file_name == name {
            break;
        }
        index += 1;
    }
    if index == len {
        return false;
    }
    inode.borrow_mut().truncate((index * per_size) as u32, per_size as u32)
}

#[derive(PartialEq, Debug)]
pub struct DirectoryInodeEntry {
    pub file_name: String,
    pub ino: u32,
}

pub struct DirectoryParser {
    pub count: usize,
    pub data: Vec<u8>,
    pub len: usize,
    pub per_size: usize,
}

impl DirectoryParser {
    pub fn new(data: &Vec<u8>) -> DirectoryParser {
        if data.len() % 14 != 0 {
            panic!("DirectoryParser: new not matched size");
        }
        DirectoryParser {
            count: 0,
            data: data.clone(),
            len: data.len(),
            per_size: 14, // 4字节 ino 10字节 name
        }
    }
    
    pub fn decode(buf: &Vec<u8>) -> Option<DirectoryInodeEntry> {
        if buf.len() != 14 {
            panic!("DirectoryParser: decode not matched size");
        }
        let mut ino = 0;
        ino += (buf[0] as u32) << 24;
        ino += (buf[1] as u32) << 16;
        ino += (buf[2] as u32) << 8;
        ino += buf[3] as u32;
        let mut len = 0;
        for byte in buf[4..14].iter() {
            if *byte != 0 {
                len += 1;
            }
        }
        if len == 0 {
            panic!("Directory: decode not available name");
        }
        Some(DirectoryInodeEntry {
            ino,
            file_name: std::str::from_utf8(&buf[4..4+len as usize]).unwrap().to_string(),
        })
    }
    
    pub fn encode(entry: &DirectoryInodeEntry) -> Option<Vec<u8>> {
        let mut res = vec![];
        let ino = entry.ino;
        res.push((ino >> 24) as u8);
        res.push((ino >> 16) as u8);
        res.push((ino >> 8) as u8);
        res.push(ino as u8);
        let mut name = entry.file_name.clone().into_bytes();
        for _ in name.len()..10 {
            name.push(0);
        }
        for i in 0..10 {
            res.push(name[i]);
        }
        Some(res)
    }
}

impl Iterator for DirectoryParser {
    type Item = DirectoryInodeEntry;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count < self.len {
            let entry = DirectoryParser::decode(&self.data[self.count..self.count+14].to_vec()).unwrap();
            self.count += 14;
            Some(entry)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use crate::inode::inode;
    use super::*;
    
    #[test]
    fn test_dirlookup() {
        let mut inode_manager = inode_manager::InodeManager::new();
        inode_manager.core_manager.borrow_mut().mount();
        let mut link = inode_manager.i_alloc();
        let stat = inode::InodeStat {
            file_type: inode::InodeFileType::Directory,
            ino: link.as_ref().unwrap().borrow().ino,
            size: 0,
            uid: 100,
            gid: 44,
            ref_cnt: 0,
            n_link: 1,
            mode: 0,
            last_accessed: (0, 0),
            last_modified: (0, 0),
            last_metadata_changed: (0, 0),
        };
        link.as_ref().unwrap().borrow_mut().modify_stat(stat);
        dir_link(link.as_mut().unwrap(), 10, "test1.txt".to_string());
        dir_link(link.as_mut().unwrap(), 11, "test2.txt".to_string());
        dir_link(link.as_mut().unwrap(), 12, "test3.txt".to_string());
        dir_unlink(link.as_mut().unwrap(), 11, "test2.txt".to_string());
        assert_eq!(dir_lookup(&link.as_ref().unwrap(), "test1.txt".to_string()), Some((10, 0)));
        assert_eq!(dir_lookup(&link.as_ref().unwrap(), "test2.txt".to_string()), None);
        assert_eq!(dir_lookup(&link.as_ref().unwrap(), "test3.txt".to_string()), Some((12, 1)));
    }

    #[test]
    fn test_dirlink() {
        let mut inode_manager = inode_manager::InodeManager::new();
        inode_manager.core_manager.borrow_mut().mount();
        let mut link = inode_manager.i_alloc();
        let stat = inode::InodeStat {
            file_type: inode::InodeFileType::Directory,
            ino: link.as_ref().unwrap().borrow().ino,
            size: 0,
            uid: 100,
            gid: 44,
            ref_cnt: 0,
            n_link: 1,
            mode: 0,
            last_accessed: (0, 0),
            last_modified: (0, 0),
            last_metadata_changed: (0, 0),
        };
        link.as_ref().unwrap().borrow_mut().modify_stat(stat);
        dir_link(link.as_mut().unwrap(), 10, "test.txt".to_string());
        let mut buf = vec![];
        link.as_ref().unwrap().borrow_mut().read_all(&mut buf);
        let entry = DirectoryParser::decode(&buf).unwrap();
        assert_eq!(entry.ino, 10);
        assert_eq!(entry.file_name, "test.txt".to_string());
        dir_unlink(link.as_mut().unwrap(), 10, "test.txt".to_string());
        link.as_ref().unwrap().borrow_mut().read_all(&mut buf);
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_directory_parser() {
        let mut data = vec![];
        let mut entries = vec![];
        entries.push(DirectoryInodeEntry {
            file_name: "a.txt".to_string(),
            ino: 10,
        });
        entries.push(DirectoryInodeEntry {
            file_name: "abc.rs".to_string(),
            ino: 11,
        });
        entries.push(DirectoryInodeEntry {
            file_name: "test.txt".to_string(),
            ino: 12,
        });
        let entry_1 = DirectoryParser::encode(&entries[0]).unwrap();
        let entry_2 = DirectoryParser::encode(&entries[1]).unwrap();
        let entry_3 = DirectoryParser::encode(&entries[2]).unwrap();
        for byte in entry_1.iter() {
            data.push(*byte);
        }
        for byte in entry_2.iter() {
            data.push(*byte);
        }
        for byte in entry_3.iter() {
            data.push(*byte);
        }
        let iter = DirectoryParser::new(&data);
        for (i, entry) in iter.enumerate(){
            assert_eq!(entry, entries[i]);
        }
    }
}