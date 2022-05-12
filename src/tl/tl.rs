use std::collections::HashMap;
use crate::util::array;
use crate::driver::disk_manager;

const MAGIC_NUMBER: u32 = 0x2222ffff;

pub struct TranslationLayer {
    pub map_table: HashMap<u32, u32>,
    pub disk_manager: disk_manager::DiskManager,
}

impl TranslationLayer {
    pub fn new() -> TranslationLayer {
        TranslationLayer {
            map_table: HashMap::new(),
            disk_manager: disk_manager::DiskManager::new(true),
        }
    }
}

impl TranslationLayer {
    pub fn read(&self, block_no: u32) -> [[u8; 4096]; 128] {
        let block_no = self.transfer(block_no);
        self.disk_manager.disk_read(block_no)
    }

    pub fn write(&mut self, address: u32, data: [u8; 4096]) {
        let mut block_no = address / 128;
        let offset = address % 128;
        block_no = self.transfer(block_no);
        let address = block_no * 128 + offset;
        self.disk_manager.disk_write(address, data);   
    }

    pub fn erase(&mut self, block_no: u32) {
        let block_no = self.transfer(block_no);
        self.disk_manager.disk_erase(block_no);
    }
}

impl TranslationLayer {
    pub fn build(&mut self, data: &array::Array2<u8>) {
        let iter = DataRegion::new(&data);
        for (lba, pba) in iter {
            self.map_table.insert(lba, pba);
        }
    }

    pub fn transfer(&self, pla: u32) -> u32 {
        if self.map_table.contains_key(&pla) {
            *self.map_table.get(&pla).unwrap()
        } else {
            pla
        }
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
    type Item = (u32, u32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.count < self.data.len() {
            let byte_1 = (self.data.get(self.count) as u32) << 24;
            let byte_2 = (self.data.get(self.count+1) as u32) << 16;
            let byte_3 = (self.data.get(self.count+2) as u32) << 8;
            let byte_4 = self.data.get(self.count+3) as u32;
            let lba = byte_1 + byte_2 + byte_3 + byte_4;
            let byte_1 = (self.data.get(self.count) as u32) << 24;
            let byte_2 = (self.data.get(self.count+1) as u32) << 16;
            let byte_3 = (self.data.get(self.count+2) as u32) << 8;
            let byte_4 = self.data.get(self.count+3) as u32;
            let pba = byte_1 + byte_2 + byte_3 + byte_4;
            self.count += 8;
            if lba == MAGIC_NUMBER && pba == MAGIC_NUMBER {
                None
            } else {
                Some((lba, pba))
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(test)]
    fn basics() {

    }
}