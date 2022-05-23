//
// Write Buffer Pool
//

use std::collections::HashMap;

// Write Cache Structure
pub struct WriteCache {
    pub capacity: usize,
    pub cache: Vec<WriteBuf>,
    pub table: HashMap<u32, bool>,
    pub sync: bool,
}

// Write Cache Simple Interface Function
impl WriteCache {
    pub fn new() -> WriteCache {
        WriteCache {
            capacity: 32,
            cache: vec![],
            sync: false,
            table: HashMap::new(),
        }
    }

    pub fn get_size(&self) -> u32 {
        self.capacity as u32
    }

    pub fn contains_address(&self, address: u32) -> bool {
        self.table.contains_key(&address)
    }

    pub fn need_sync(&self) -> bool {
        self.sync
    }

    pub fn sync(&mut self) {
        self.sync = false;
        self.cache.clear();
        self.table.clear();
    }
}

// Write Cache Main Interface Function
impl WriteCache {
    /// Write page to cache
    /// params:
    /// address - write page's address
    /// data - write data
    /// return:
    /// ()
    pub fn write(&mut self, address: u32, data: [u8; 4096]) {
        let index = self.cache.len();
        if index == self.capacity {
            panic!("WriteCache: write has too much buf");
        }
        if index == self.capacity - 1 {
            self.sync = true;
        }
        let buf = WriteBuf {
            address,
            data
        };
        if !self.table.contains_key(&address) {
            self.cache.push(buf);
            self.table.insert(address, true);
        } else {
            for index in 0..self.cache.len() {
                if self.cache[index].address == address {
                    self.cache[index] = buf;
                    break;
                }
            }
        }
    }

    /// Get page from cache
    /// params:
    /// address - read page's address
    /// return:
    /// page data
    pub fn read(&self, address: u32) -> Option<[u8; 4096]> {
        if !self.table.contains_key(&address) {
            return None;
        }
        for index in 0..self.cache.len() {
            if self.cache[index].address == address {
                return Some(self.cache[index].data);
            }
        }
        panic!("WriteBuf: read internal error");
    }

    /// Get all page from cache
    /// params:
    /// ()
    /// return:
    /// all page data
    pub fn get_all(&self) -> Vec<(u32, [u8; 4096])> {
        let mut buf = vec![];
        for entry in self.cache.iter() {
            buf.push((entry.address, entry.data));
        }
        buf
    }

    /// Recall write in cache
    /// params:
    /// address - recall page's address
    /// return:
    /// ()
    pub fn recall_write(&mut self, address: u32) {
        if self.table.contains_key(&address) {
            for index in 0..self.cache.len() {
                if self.cache[index].address == address {
                    self.cache.remove(index);
                    break;
                }
            }
            self.table.remove(&address);
        }
    }
}

// Simple Buf Structure
#[derive(Clone, Copy)]
pub struct WriteBuf {
    pub address: u32,
    pub data: [u8; 4096],
}

impl WriteBuf {
    fn new(address: u32, data: [u8; 4096]) -> WriteBuf {
        WriteBuf {
            address: address,
            data,
        }
    }
}

// Write Buffer Pool Module Test
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basics() {
        let mut write_buf = WriteCache::new();
        for i in 0..32 {
            write_buf.write(i, [0; 4096]);
        }
        assert_eq!(write_buf.need_sync(), true);
        write_buf.sync();
        assert_eq!(write_buf.need_sync(), false);
        assert_eq!(write_buf.cache.len(), 0);
    }
}