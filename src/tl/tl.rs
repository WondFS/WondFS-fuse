use std::collections::HashMap;
use std::time::{UNIX_EPOCH, SystemTime, Duration};
use crate::util::array;
use crate::driver::disk_manager;
use crate::tl::check_center;
use crate::tl::tl_helper::*;
use crate::write_buf;

const MAGIC_NUMBER_1: u32 = 0x2222ffff; // 标志坏块映射块
const MAGIC_NUMBER_2: u32 = 0x3333aaaa; // 标志校验存储块

pub struct TranslationLayer {
    pub disk_manager: disk_manager::DiskManager,
    pub write_cache: write_buf::WriteCache,
    pub used_table: HashMap<u32, bool>,
    pub map_v_table: HashMap<u32, u32>,
    pub sign_block_map: HashMap<u32, u32>,
    pub sign_offset_map: HashMap<u32, u32>,
    pub block_num: u32,
    pub err_block_num: u32,
    pub use_max_block_no: u32,
    pub max_block_no: u32,
    pub table_block_no: u32,
    pub sign_block_no: u32,
    pub sign_block_offset: u32,
}

impl TranslationLayer {
    pub fn new() -> TranslationLayer {
        TranslationLayer {
            disk_manager: disk_manager::DiskManager::new(true),
            write_cache: write_buf::WriteCache::new(),
            map_v_table: HashMap::new(),
            used_table: HashMap::new(),
            sign_block_map: HashMap::new(),
            sign_offset_map: HashMap::new(),
            block_num: 32,
            err_block_num: 0,
            use_max_block_no: 27,
            max_block_no: 31,
            table_block_no: 28,
            sign_block_no: 29,
            sign_block_offset: 0,
        }
    }
}

impl TranslationLayer {
    pub fn read(&mut self, block_no: u32) -> [[u8; 4096]; 128] {
        let start_index = block_no * 128;
        let end_index = (block_no + 1) * 128;
        let mut exist_indexs = vec![];
        for index in start_index..end_index {
            if self.write_cache.contains_address(index) {
                exist_indexs.push(index);
            }
        }
        let mut block_data = transfer(self.disk_manager.disk_read(self.transfer(block_no)));
        for index in exist_indexs.into_iter() {
            let data = self.write_cache.read(index).unwrap();
            block_data.set(index - start_index, data);
        }
        let mut data = reverse(&block_data);
        self.check_block(block_no, &mut data);
        data
    }

    pub fn write(&mut self, address: u32, data: [u8; 4096]) {
        self.write_cache.write(address, data);
        if !self.write_cache.need_sync() {
            return;
        }
        let data = self.write_cache.get_all();
        
        for (address, data) in data.into_iter() {
            let mut block_no = address / 128;
            let offset = address % 128;
            block_no = self.transfer(block_no);
            let address = block_no * 128 + offset;
            self.disk_manager.disk_write(address, data);   
        }
        self.write_cache.sync();
    }

    pub fn erase(&mut self, block_no: u32) {
        let start_index = block_no * 128;
        let end_index = (block_no + 1) * 128;
        for index in start_index..end_index {
            self.write_cache.recall_write(index);
        }
        let block_no = self.transfer(block_no);
        self.disk_manager.disk_erase(block_no);
    }
}

impl TranslationLayer {
    pub fn init(&mut self) {
        for block_no in self.use_max_block_no + 1..=self.max_block_no {
            self.init_with_block(block_no, transfer(self.disk_manager.disk_read(block_no)));
        }
    }

    pub fn init_with_block(&mut self, block_no: u32, data: array::Array1<[u8; 4096]>) {
        let block_type = judge_block_type(&data);
        match block_type {
            BlockType::MappingTable => {
                let iter = MapDataRegion::new(&truncate_array_1_to_array_2(data));
                for entry in iter {
                    self.map_v_table.insert(entry.0, entry.1);
                    self.used_table.insert(entry.1, true);
                }
                self.used_table.insert(block_no, true);
                self.table_block_no = block_no;
            },
            BlockType::Signature => {
                let iter = SignDataRegion::new(&truncate_array_1_to_array_2(data));
                let mut len = 0;
                for (index, entry) in iter.enumerate() {
                    let address = check_center::CheckCenter::extract_address(&entry.to_vec());
                    if self.sign_block_map.contains_key(&address) {
                        *self.sign_block_map.get_mut(&address).unwrap() = block_no;
                        *self.sign_offset_map.get_mut(&address).unwrap() = index as u32;
                    } else {
                        self.sign_block_map.insert(address, block_no);
                        self.sign_offset_map.insert(address, index as u32);
                    }
                    len += 1;
                }
                self.used_table.insert(block_no, true);
                self.sign_block_no = block_no;
                self.sign_block_offset = len;
            },
            _ => (),
        }
    }

    pub fn check_block(&mut self, block_no: u32, data: &mut [[u8; 4096]; 128]) -> bool {
        let mut flag = true;
        for (index, page) in data.clone().iter().enumerate() {
            let address = block_no * 128 + index as u32;
            let signature = self.get_address_sign(address);
            let ret = check_center::CheckCenter::check(page, &signature);
            if ret.0 == false {
                if ret.2 == None {
                    flag = false;
                    break;
                } else {
                    data[index] = ret.2.unwrap();
                }
            }
        }
        if !flag {
            warn!("TranslationLayer: {} block has broken", block_no);
            let new_block_no = self.find_next_block();
            self.used_table.insert(new_block_no, true);
            self.map_v_table.insert(block_no, new_block_no);
            self.err_block_num += 1;
            return false;
        }
        true
    }

    pub fn write_sign(&mut self, data: Vec<(u32, [u8;4096])>) {
        let mut page_data = [0; 4096];
        for (index, data) in data.iter().enumerate() {
            let signature = self.set_address_sign(&data.1, data.0);
            let start_index = index * 128;
            for (index, byte) in signature.iter().enumerate() {
                page_data[start_index + index] = *byte;
            }
        }
        if self.sign_block_offset / 32 > 127 {
            self.sign_block_no = self.find_next_block();
            self.sign_block_offset = 0;
        }
        let address = self.sign_block_no * 128 + self.sign_block_offset / 32;
        self.sign_block_offset += 32;
        self.disk_manager.disk_write(address, page_data);
    }

    pub fn transfer(&self, pla: u32) -> u32 {
        if self.map_v_table.contains_key(&pla) {
            *self.map_v_table.get(&pla).unwrap()
        } else {
            pla
        }
    }

    pub fn get_address_sign(&self, address: u32) -> Vec<u8> {
        let address = self.sign_block_map.get(&address);
        if address.is_none() {
            panic!("TranslationLayer: get sign no available adderss");
        }
        let address = address.unwrap();
        let offset = self.sign_offset_map.get(&address);
        if offset.is_none() {
            panic!("TranslationLayer: get sign no available address");
        }
        let offset = offset.unwrap();
        let data = self.disk_manager.disk_read(address / 128)[(address % 128) as usize];
        data[(offset*128) as usize..(offset*128+128) as usize].to_vec()
    }

    pub fn set_address_sign(&self, data: &[u8; 4096], address: u32) -> Vec<u8> {
        let sign_type = self.choose_sign_type();
        check_center::CheckCenter::sign(data, address, sign_type)
    }

    pub fn choose_sign_type(&self) -> check_center::CheckType {
        check_center::CheckType::Crc32
    }

    pub fn find_next_block(&self) -> u32 {
        for block_no in self.use_max_block_no+1..self.max_block_no {
            if block_no == self.table_block_no || block_no == self.sign_block_no {
                continue;
            }
            if self.used_table.contains_key(&block_no) {
                continue;
            }
            return block_no;
        }
        panic!("TranslationLayer: No available block to map")
    }
}

pub struct MapDataRegion {
    count: u32,
    data: array::Array1<u8>,
    
}

impl MapDataRegion {
    pub fn new(data: &array::Array2<u8>) -> MapDataRegion {
        if data.size() != [128, 4096] {
            panic!("MapDataRegion: new not matched size");
        }
        let mut arr = array::Array1::<u8>::new(data.len());
        for (index, byte) in data.iter().skip(8).enumerate() {
            arr.set(index as u32, byte);
        }
        MapDataRegion {
            count: 0,
            data: arr,
        }
    }
}

impl Iterator for MapDataRegion {
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
            if lba == 0 && pba == 0 {
                None
            } else {
                Some((lba, pba))
            }
        } else {
            None
        }
    }
}

pub struct SignDataRegion {
    count: u32,
    data: array::Array1<u8>,
}

impl SignDataRegion {
    pub fn new(data: &array::Array2<u8>) -> SignDataRegion {
        if data.size() != [128, 4096] {
            panic!("SignDataRegion: new not matched size");
        }
        let mut arr = array::Array1::<u8>::new(data.len());
        for (index, byte) in data.iter().skip(8).enumerate() {
            arr.set(index as u32, byte);
        }
        SignDataRegion {
            count: 0,
            data: arr,
        }
    }
}

impl Iterator for SignDataRegion {
    type Item = [u8; 128];
    fn next(&mut self) -> Option<Self::Item> {
        if self.count < self.data.len() {
            let mut data = [0; 128];
            for i in self.count..self.count+128 {
                data[(i - self.count) as usize] = self.data.get(i);
            }
            self.count += 128;
            if data == [0; 128] {
                None
            } else {
                Some(data)
            }
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
        let mut tl = TranslationLayer::new();
        tl.init();

        let data = [1; 4096];
        tl.write(100, data);
        let data = tl.read(0);
        // assert_eq!(data[100], [1; 4096]);

        // tl.erase(0);
        // let data = tl.read(0); 
        // assert_eq!(data[100], [0; 4096]);
    }
}