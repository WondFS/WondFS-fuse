use std::collections::HashMap;
use crate::util::array;
use crate::driver::disk_manager;
use crate::tl::check_center;
use crate::tl::tl_helper::*;
use crate::write_buf;

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
        let mut tl = TranslationLayer {
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
        };
        tl.init();
        tl
    }
}

impl TranslationLayer {
    pub fn read(&mut self, block_no: u32) -> array::Array1<[u8; 4096]> {
        if block_no > self.use_max_block_no {
            panic!("TranslationLayer: read can't read reserved region directly even more");
        }
        let start_index = block_no * 128;
        let end_index = (block_no + 1) * 128;
        let mut exist_indexs = vec![];
        let mut should_check = vec![];
        for index in start_index..end_index {
            should_check.push(true);
            if self.write_cache.contains_address(index) {
                exist_indexs.push(index);
                should_check[index as usize] = false;
            }
        }
        let map_block_no = self.transfer(block_no);
        trace!("TranslationLayer: read block block_no: {}, map to block_no: {} ", block_no, map_block_no);
        let mut block_data = transfer(self.disk_manager.disk_read(map_block_no));
        for index in exist_indexs.into_iter() {
            let data = self.write_cache.read(index).unwrap();
            block_data.set(index - start_index, data);
        }
        self.check_block(block_no, &mut block_data, &should_check);
        block_data
    }

    pub fn write(&mut self, address: u32, data: [u8; 4096]) {
        self.write_cache.write(address, data);
        if !self.write_cache.need_sync() {
            return;
        }
        trace!("TranslationLayer: write cache full, need clear");
        let data = self.write_cache.get_all();
        self.write_sign(&data);
        for (address, data) in data.into_iter() {
            let  block_no = address / 128;
            let offset = address % 128;
            let map_block_no = self.transfer(block_no);
            let map_address = map_block_no * 128 + offset;
            trace!("TranslationLayer: write page address: {}, map to adderss: {}", address, map_address);
            self.disk_manager.disk_write(map_address, data);   
        }
        trace!("TranslationLayer: write cache had clear");
        self.write_cache.sync();
    }

    pub fn erase(&mut self, block_no: u32) {
        let start_index = block_no * 128;
        let end_index = (block_no + 1) * 128;
        for index in start_index..end_index {
            self.write_cache.recall_write(index);
            if self.sign_block_map.contains_key(&index) {
                self.sign_block_map.remove(&index);
                self.sign_offset_map.remove(&index);
            }
        }
        let map_block_no = self.transfer(block_no);
        trace!("TranslationLayer: erase block block_no: {}, map to block_no: {}", block_no, map_block_no);
        self.disk_manager.disk_erase(map_block_no);
    }
}

impl TranslationLayer {
    pub fn init(&mut self) {
        trace!("TranslationLayer: init with block range block_no: {} - block_no: {}", self.use_max_block_no + 1, self.max_block_no);
        for block_no in self.use_max_block_no + 1..=self.max_block_no {
            self.init_with_block(block_no, transfer(self.disk_manager.disk_read(block_no)));
        }
    }

    pub fn init_with_block(&mut self, block_no: u32, data: array::Array1<[u8; 4096]>) {
        let block_type = judge_block_type(&data);
        match block_type {
            BlockType::MappingTable => {
                trace!("TranslationLayer: init with mapping table block block_no: {}", block_no);
                let iter = MapDataRegion::new(&data);
                for entry in iter {
                    self.map_v_table.insert(entry.0, entry.1);
                    self.used_table.insert(entry.1, true);
                }
                self.used_table.insert(block_no, true);
                self.table_block_no = block_no;
            },
            BlockType::Signature => {
                trace!("TranslationLayer: init with signature block block_no: {}", block_no);
                let iter = SignDataRegion::new(&data);
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

    pub fn check_block(&mut self, block_no: u32, data: &mut array::Array1<[u8; 4096]>, should_check: &Vec<bool>) -> bool {
        let mut flag = true;
        for (index, page) in data.dup().iter().enumerate() {
            if !should_check[index] {
                continue;
            }
            let address = block_no * 128 + index as u32;
            let signature = self.get_address_sign(address);
            if signature.is_none() {
                continue;
            }
            let ret = check_center::CheckCenter::check(&page, &signature.as_ref().unwrap());
            if ret.0 == false {
                if ret.2 == None {
                    println!("{:?} {}", signature.unwrap(), index);
                    flag = false;
                    break;
                } else {
                    data.set(index as u32, ret.2.unwrap());
                }
            }
        }
        if !flag {
            warn!("TranslationLayer: block block_no: {}, has broken", block_no);
            let new_block_no = self.find_next_block();
            self.used_table.insert(new_block_no, true);
            self.map_v_table.insert(block_no, new_block_no);
            self.err_block_num += 1;
            self.sync_map_v_table();
            return false;
        }
        true
    }

    pub fn write_sign(&mut self, data: &Vec<(u32, [u8;4096])>) {
        if data.len() != 32 {
            panic!("TranslationLayer: write sign no available size");
        }
        let mut page_data = [0; 4096];
        if self.sign_block_offset / 32 == 127 {
            if !self.used_table.contains_key(&self.sign_block_no) {
                self.used_table.insert(self.sign_block_no, true);
            }
            self.sign_block_no = self.find_next_block();
            self.sign_block_offset = 0;
        }
        let address = self.sign_block_no * 128 + self.sign_block_offset / 32;
        for (index, data) in data.iter().enumerate() {
            let signature = self.set_address_sign(&data.1, data.0);
            let start_index = index * 128;
            for (index, byte) in signature.iter().enumerate() {
                page_data[start_index + index] = *byte;
            }
            if self.sign_block_map.contains_key(&data.0) {
                *self.sign_block_map.get_mut(&data.0).unwrap() = self.sign_block_no;
                *self.sign_offset_map.get_mut(&data.0).unwrap() = self.sign_block_offset + index as u32;
            } else {
                self.sign_block_map.insert(data.0, self.sign_block_no);
                self.sign_offset_map.insert(data.0,  self.sign_block_offset + index as u32);
            }
        }
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

    pub fn get_address_sign(&self, address: u32) -> Option<Vec<u8>> {
        let sign_address = self.sign_block_map.get(&address);
        if sign_address.is_none() {
            return None;
        }
        let sign_address = sign_address.unwrap();
        let offset = self.sign_offset_map.get(&address);
        if offset.is_none() {
            panic!("TranslationLayer: get sign internal error");
        }
        let offset = offset.unwrap();
        let data = self.disk_manager.disk_read(*sign_address)[(offset / 32) as usize];
        let ret = data[(offset % 32 *128) as usize..(offset % 32 *128+128) as usize].to_vec();
        Some(ret)
    }

    pub fn set_address_sign(&self, data: &[u8; 4096], address: u32) -> Vec<u8> {
        let sign_type = self.choose_sign_type();
        let sign = check_center::CheckCenter::sign(data, address, sign_type);
        sign
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
            trace!("TranslationLayer: find next block to use block_no: {}", block_no);
            return block_no;
        }
        panic!("TranslationLayer: No available block to map")
    }


    pub fn sync_map_v_table(&mut self) {
        let mut data = array::Array1::<u8>::new(128 * 4096);
        data.init(0);
        let mut index = 0;
        for (key, value) in &self.map_v_table {
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
        self.write_table_block(&data);
    }

    pub fn write_table_block(&mut self, data: &array::Array1::<u8>) {
        self.disk_manager.disk_erase(self.table_block_no);
        let mut index = 0;
        while index < 128 {
            let start_index = 4096 * index;
            let end_index = (index + 1) * 4096;
            let mut page = [0; 4096];
            for index in start_index..end_index {
                page[index - start_index] = data.get(index as u32);
            }
            self.disk_manager.disk_write(self.table_block_no * 128 + index as u32, page);
            index += 1;
        }
    }
}

pub struct MapDataRegion<'a> {
    count: u32,
    data: &'a array::Array1<[u8; 4096]>,
}

impl MapDataRegion<'_> {
    pub fn new(data: &array::Array1<[u8; 4096]>) -> MapDataRegion {
        if data.len() != 128 {
            panic!("MapDataRegion: new not matched size");
        }
        MapDataRegion {
            count: 8,
            data,
        }
    }
}

impl Iterator for MapDataRegion<'_> {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.count < 128 * 4096 {
            let byte_1 = (self.data.get(self.count / 4096)[(self.count % 4096) as usize] as u32) << 24;
            let byte_2 = (self.data.get((self.count + 1) / 4096)[((self.count + 1) % 4096) as usize] as u32) << 16;
            let byte_3 = (self.data.get((self.count + 2) / 4096)[((self.count + 2) % 4096) as usize] as u32) << 8;
            let byte_4 = self.data.get((self.count + 3) / 4096)[((self.count + 3) % 4096) as usize] as u32;
            let lba = byte_1 + byte_2 + byte_3 + byte_4;
            let byte_1 = (self.data.get((self.count + 4) / 4096)[((self.count + 4) % 4096) as usize] as u32) << 24;
            let byte_2 = (self.data.get((self.count + 5) / 4096)[((self.count + 5) % 4096) as usize] as u32) << 16;
            let byte_3 = (self.data.get((self.count + 6) / 4096)[((self.count + 6) % 4096) as usize] as u32) << 8;
            let byte_4 = self.data.get((self.count + 7) / 4096)[((self.count + 7) % 4096) as usize] as u32;
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

pub struct SignDataRegion<'a> {
    count: u32,
    data: &'a array::Array1<[u8; 4096]>,
}

impl SignDataRegion<'_> {
    pub fn new(data: &array::Array1<[u8; 4096]>) -> SignDataRegion {
        if data.len() != 128 {
            panic!("SignDataRegion: new not matched size");
        }
        SignDataRegion {
            count: 0,
            data,
        }
    }
}

impl Iterator for SignDataRegion<'_> {
    type Item = [u8; 128];
    fn next(&mut self) -> Option<Self::Item> {
        if self.count < 128 * 4096 {
            let mut data = [0; 128];
            for i in self.count..self.count+128 {
                data[(i - self.count) as usize] = self.data.get(i / 4096)[(i % 4096) as usize];
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

        let data = [1; 4096];
        for i in 0..300 {
            tl.write(i, data);
        }
        let data = tl.read(0);
        assert_eq!(data.get(100), [1; 4096]);

        tl.erase(0);
        let data = tl.read(0); 
        assert_eq!(data.get(100), [0; 4096]);
    }
}