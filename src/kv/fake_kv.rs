use std::collections::HashMap;
use crate::kv::raw_inode;

pub struct FakeKV {
    pub next_ino: u32,
    pub map: HashMap<u32, raw_inode::RawInode>,
}

impl FakeKV {
    pub fn new() -> FakeKV {
        FakeKV {
            next_ino: 1,
            map: HashMap::new(),
        }
    }

    pub fn get_inode(&self, ino: u32) -> raw_inode::RawInode {
        if !self.map.contains_key(&ino) {
            panic!("FakeKV: get no that inode");
        }
        let raw_inode = self.map.get(&ino).unwrap();
        let mut data = vec![];
        for entry in raw_inode.data.iter() {
            data.push(*entry);
        }
        raw_inode::RawInode {
            ino,
            data,
            uid: raw_inode.uid,
            gid: raw_inode.gid,
            size: raw_inode.size,
            n_link: raw_inode.n_link,
            ref_cnt: raw_inode.ref_cnt,
            file_type: raw_inode.file_type,
            mode: todo!(),
            last_accessed: todo!(),
            last_modified: todo!(),
            last_metadata_changed: todo!(),
        }
    }

    pub fn update_inode(&mut self, inode: raw_inode::RawInode) {
        let ino = inode.ino;
        if !self.map.contains_key(&ino) {
            panic!("FakeKV: update no that inode");
        }
        *self.map.get_mut(&ino).unwrap() = inode;
    }

    pub fn delete_inode(&mut self, ino: u32) {
        if !self.map.contains_key(&ino) {
            panic!("FakeKV: delete no that inode");
        }
        self.map.remove(&ino);
    }

    pub fn allocate_inode(&mut self) -> raw_inode::RawInode {
        let ino = self.next_ino;
        self.next_ino += 1;
        let raw_inode = raw_inode::RawInode {
            ino,
            uid: 0,
            gid: 0,
            size: 0,
            n_link: 1,
            ref_cnt: 0,
            file_type: 0,
            data: vec![],
            mode: 0,
            last_accessed: (0, 0),
            last_modified: (0, 0),
            last_metadata_changed: (0, 0),
        };
        self.map.insert(ino, raw_inode);
        let raw_inode = raw_inode::RawInode {
            ino,
            uid: 0,
            gid: 0,
            size: 0,
            n_link: 1,
            ref_cnt: 0,
            file_type: 0,
            data: vec![],
            mode: 0,
            last_accessed: (0, 0),
            last_modified: (0, 0),
            last_metadata_changed: (0, 0),
        };
        raw_inode
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basics() {
        let mut kv = FakeKV::new();
        
        let mut inode = kv.allocate_inode();
        let ino = inode.ino;
        inode.gid = 100;
        inode.file_type = 1;
        kv.update_inode(inode);

        let inode = kv.get_inode(ino);
        assert_eq!(inode.gid, 100);
        assert_eq!(inode.file_type, 1);
    }
}