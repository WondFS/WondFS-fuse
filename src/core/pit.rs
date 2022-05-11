use std::collections::HashMap;
use crate::util::array::{self, Array2};

pub struct PIT {
    pub table: HashMap<u32, u32>,  // page -> ino
    pub sync: bool,                // true 需要持久化到磁盘中
    pub is_op: bool,               // true 等调用end_op才持久化到磁盘中
}

impl PIT {
    pub fn new() -> PIT {
        PIT {
            table: HashMap::new(),
            sync: false,
            is_op: false,
        }
    }

    pub fn init_page(&mut self, address: u32, status: u32) {
        if self.table.contains_key(&address) {
            panic!("PIT: init page has exist");
        }
        self.table.insert(address, status);
    }

    pub fn get_page(&self, address: u32) -> u32 {
        if !self.table.contains_key(&address) {
            panic!("PIT: get not that page");
        }
        self.table.get(&address).unwrap().clone()
    }

    pub fn set_page(&mut self, address: u32, status: u32) {
        if !self.table.contains_key(&address) {
            self.table.insert(address, status);
            self.sync = true;
            return;
        }
        *self.table.get_mut(&address).unwrap() = status;
        self.sync = true;
    }

    pub fn delete_page(&mut self, address: u32) {
        if !self.table.contains_key(&address) {
            panic!("PIT: delete not that page");
        }
        self.table.remove(&address).unwrap();
        self.sync = true;
    }

    pub fn clean_page(&mut self, address: u32) {
        if self.table.contains_key(&address) {
            self.table.remove(&address).unwrap();
            self.sync = true;
        }
    }

    pub fn encode(&self) -> Array2<u8> {
        let mut res = array::Array1::<u32>::new(128 * 4096 / 4);
        res.init(0);
        for (key, value) in &self.table {
            res.set(*key, *value);
        }
        let mut temp = array::Array1::<u8>::new(128 * 4096);
        temp.init(0);
        for (index, value) in res.iter().enumerate() {
            let byte_1 = (value >> 24) as u8;
            let byte_2 = (value >> 16) as u8;
            let byte_3 = (value >> 8) as u8;
            let byte_4 = value as u8;
            let start_index = index * 4;
            temp.set(start_index as u32, byte_1);
            temp.set((start_index + 1) as u32, byte_2);
            temp.set((start_index + 2) as u32, byte_3);
            temp.set((start_index + 3) as u32, byte_4);
        }
        let mut data = array::Array2::<u8>::new(128, 4096);
        data.init(0);
        for (index, value) in temp.iter().enumerate() {
            let i = index / 4096;
            let j = index % 4096;
            data.set(i as u32, j as u32, value);
        }
        data
    }

    pub fn need_sync(&self) -> bool {
        if self.is_op {
            return false;
        }
        self.sync
    }

    pub fn sync(&mut self) {
        self.sync = false;
    }

    pub fn begin_op(&mut self) {
        self.is_op = true;
    }

    pub fn end_op(&mut self) {
        self.is_op = false;
    }

}

pub struct DataRegion {
    count: u32,
    data: array::Array1<u8>,
}

impl DataRegion {
    pub fn new(data: &array::Array2<u8>) -> DataRegion {
        if data.size() != [128, 4096] {
            panic!("DataRegion: new not matched size");
        }
        let mut arr = array::Array1::<u8>::new(data.len());
        for (index, byte) in data.iter().enumerate() {
            arr.set(index as u32, byte);
        }
        DataRegion {
            count: 0,
            data: arr,
        }
    }
}

impl Iterator for DataRegion {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count < self.data.len() {
            let byte_1 = (self.data.get(self.count) as u32) << 24;
            let byte_2 = (self.data.get(self.count+1) as u32) << 16;
            let byte_3 = (self.data.get(self.count+2) as u32) << 8;
            let byte_4 = self.data.get(self.count+3) as u32;
            self.count += 4;
            Some(byte_1 + byte_2 + byte_3 + byte_4)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basics() {
        let mut pit = PIT::new();
        let mut data = array::Array2::<u8>::new(128, 4096);
        data.init(0);
        data.set(100, 312, 234);
        data.set(11, 232, 67);
        data.set(121, 2332, 123);
        let iter = DataRegion::new(&data);
        for (index, ino) in iter.enumerate() {
            if ino != 0 {
                pit.init_page(index as u32, ino);
            }
        }
        assert_eq!(pit.encode(), data);
        assert_eq!(pit.need_sync(), false);
        pit.set_page(200, 100);
        assert_eq!(pit.get_page(200), 100);
        assert_eq!(pit.need_sync(), true);
    }
}
