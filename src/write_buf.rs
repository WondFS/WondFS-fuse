use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct WriteBuf {
    pub address: u32,
    pub data: [u8; 4096],
}

pub struct WriteCache {
    pub capacity: usize,
    pub cache: Vec<WriteBuf>,
    pub table: HashMap<u32, bool>,
    pub sync: bool,
}

impl WriteCache {
    pub fn new() -> WriteCache {
        WriteCache {
            capacity: 32,
            cache: vec![],
            sync: false,
            table: HashMap::new(),
        }
    }

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

    pub fn get_all(&self) -> Vec<(u32, [u8; 4096])> {
        let mut buf = vec![];
        for entry in self.cache.iter() {
            buf.push((entry.address, entry.data));
        }
        buf
    }

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