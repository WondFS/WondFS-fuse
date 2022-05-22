use std::collections::HashMap;
use crate::util::array;

pub enum JournalType {
    GC,
    None,
}

const MAGIC_NUMBER: u32 = 0x7777ffff;

pub struct Journal {
    pub table: HashMap<u32, u32>,
    pub sync: bool,
    pub is_op: bool,
    pub erase_block_no: u32,
}

impl Journal {
    pub fn new() -> Journal {
        Journal {
            table: HashMap::new(),
            sync: false,
            is_op: false,
            erase_block_no: 0,
        }
    }

    pub fn get_erase_block_no(&self) -> u32 {
        self.erase_block_no
    }

    pub fn set_erase_block_no(&mut self, block_no: u32) {
        self.erase_block_no = block_no;
    }

    pub fn set_journal(&mut self, o_address: u32, address: u32) {
        if self.table.contains_key(&o_address) {
            panic!("Journal: set journal has conflicts");
        }
        self.table.insert(o_address, address);
    }

    pub fn encode(&self) -> array::Array1::<u8> {
        let mut data = array::Array1::<u8>::new(128 * 4096);
        data.init(0);
        data.set(0, 0x77);
        data.set(1, 0x77);
        data.set(2, 0xff);
        data.set(3, 0xff);
        let byte_1 = (self.erase_block_no >> 24) as u8;
        let byte_2 = (self.erase_block_no >> 16) as u8;
        let byte_3 = (self.erase_block_no >> 8) as u8;
        let byte_4 = self.erase_block_no as u8;
        data.set(4, byte_1);
        data.set(5, byte_2);
        data.set(6, byte_3);
        data.set(7, byte_4);
        let mut index = 0;
        for (key, value) in &self.table {
            let start_index = 8 + index * 8;
            let byte_1 = (*key >> 24) as u8;
            let byte_2 = (*key >> 16) as u8;
            let byte_3 = (*key >> 8) as u8;
            let byte_4 = *key as u8;
            data.set(start_index, byte_1);
            data.set(start_index + 1, byte_2);
            data.set(start_index + 2, byte_3);
            data.set(start_index + 3, byte_4);
            let byte_1 = (*value >> 24) as u8;
            let byte_2 = (*value >> 16) as u8;
            let byte_3 = (*value >> 8) as u8;
            let byte_4 = *value as u8;
            data.set(start_index + 4, byte_1);
            data.set(start_index + 5, byte_2);
            data.set(start_index + 6, byte_3);
            data.set(start_index + 7, byte_4);
            index += 1;
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

    pub fn clear(&mut self) {
        self.erase_block_no = 0;
        self.table.clear();
    }

    pub fn begin_op(&mut self) {
        self.is_op = true;
    }

    pub fn end_op(&mut self) {
        self.is_op = false;
    }
}

pub struct DataRegion<'a> {
    count: u32,
    data: &'a array::Array1<[u8; 4096]>,   
}

impl DataRegion<'_> {
    pub fn new(data: &array::Array1::<[u8; 4096]>) -> DataRegion {
        if data.len() != 128 {
            panic!("DataRegion: new not matched size");
        }
        DataRegion {
            count: 8,
            data,
        }
    }
}

impl Iterator for DataRegion<'_> {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.count < 128 * 4096 {
            let byte_1 = (self.data.get(self.count / 4096)[(self.count % 4096) as usize] as u32) << 24;
            let byte_2 = (self.data.get((self.count + 1) / 4096)[((self.count + 1) % 4096) as usize] as u32) << 16;
            let byte_3 = (self.data.get((self.count + 2) / 4096)[((self.count + 2) % 4096) as usize] as u32) << 8;
            let byte_4 = self.data.get((self.count + 3) / 4096)[((self.count + 3) % 4096) as usize] as u32;
            let o_address = byte_1 + byte_2 + byte_3 + byte_4;
            let byte_1 = (self.data.get((self.count + 4) / 4096)[((self.count + 4) % 4096) as usize] as u32) << 24;
            let byte_2 = (self.data.get((self.count + 5) / 4096)[((self.count + 5) % 4096) as usize] as u32) << 16;
            let byte_3 = (self.data.get((self.count + 6) / 4096)[((self.count + 6) % 4096) as usize] as u32) << 8;
            let byte_4 = self.data.get((self.count + 7) / 4096)[((self.count + 7) % 4096) as usize] as u32;
            let address = byte_1 + byte_2 + byte_3 + byte_4;
            self.count += 8;
            if o_address == 0 && address == 0 {
                None
            } else {
                Some((o_address, address))
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::UNIX_EPOCH;

    use super::*;

    #[test]
    fn basics() {

    }
}