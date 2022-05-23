//
// Buf Layer
//

use crate::util::lru_cache;
use crate::tl::tl;
use crate::util::array;

// Buf Layer Main Structure
pub struct BufCache {
    pub capacity: usize,
    pub cache: lru_cache::LRUCache<Buf>,
    pub translation_layer: tl::TranslationLayer,
}

// Buf Layer Simple Interface Function
impl BufCache {
    pub fn new() -> BufCache {
        let capacity = 1024;
        BufCache {
            capacity: capacity as usize,
            cache: lru_cache::LRUCache::new(capacity as usize),
            translation_layer: tl::TranslationLayer::new(),
        }
    }

    pub fn init(&mut self) {
        self.translation_layer.init();
    }
}

// Buf Layer Main Interface Function
impl BufCache {
    /// Read page
    /// params:
    /// dev - device number
    /// address - read page's address
    /// return:
    /// page data
    pub fn read(&mut self, dev: u8, address: u32) -> [u8; 4096] {
        let data = self.get_data(address);
        if data.is_some() {
            return data.unwrap();
        }
        let block_no = address / 128;
        let data = self.translation_layer.read(block_no);
        for (index, page) in data.iter().enumerate() {
            self.put_data(address + index as u32, page);
        }
        self.get_data(address).unwrap()
    }

    /// Write page
    /// params:
    /// dev - device number
    /// address - write page's address
    /// data - write data
    /// return:
    /// ()
    pub fn write(&mut self, dev: u8, address: u32, data: [u8; 4096]) {
        self.put_data(address, data);
        self.translation_layer.write(address, data);
    }

    /// Write block directly, bypasses write cache
    /// params:
    /// dev - device number
    /// block_no - write block's block number
    /// data - write data
    /// return:
    /// ()
    pub fn write_block_driect(&mut self, dev: u8, block_no: u32, data: array::Array1::<[u8; 4096]>) {
        if data.len() != 128 {
            panic!("BufCache: write block directly no available data size");
        }
        let start_address = block_no * 128;
        for i in 0..data.len() {
            let address = start_address + i;
            self.put_data(address, data.get(i));
        }
        self.translation_layer.write_block_direct(block_no, data);
    }

    /// Erase block
    /// params:
    /// dev - device number
    /// block_no - erase block's block number
    /// return:
    /// ()
    pub fn erase(&mut self, dev: u8, block_no: u32) {
        let start_address = block_no * 128;
        let end_address = (block_no + 1) * 128;
        for address in start_address..end_address {
            self.remove_data(address);
        }
        self.translation_layer.erase(block_no);
    }
}

// Buf Layer Internal Function
impl BufCache {
    fn get_data(&mut self, address: u32) -> Option<[u8; 4096]> {
        let data = self.cache.get(address);
        if data.is_some() {
            return Some(data.as_deref().unwrap().data);
        }
        None
    }

    fn put_data(&mut self, address: u32, data: [u8; 4096]) {
        let buf = Buf::new(address, data);
        self.cache.put(address, buf);
    }

    fn remove_data(&mut self, address: u32) {
        self.cache.remove(address);
    }
}

// Simple Buf Structure
#[derive(Clone, Copy)]
pub struct Buf {
    address: u32,
    data: [u8; 4096],
}

impl Buf {
    fn new(address: u32, data: [u8; 4096]) -> Buf {
        Buf {
            address: address,
            data,
        }
    }
}

// Buf Layer Module Test
#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    fn basics() {
        let mut cache = BufCache::new();

        let data = [1; 4096];        
        cache.put_data(100, data);
        assert_eq!(cache.get_data(100).unwrap(), [1; 4096]);
        cache.remove_data(100);
        assert_eq!(cache.get_data(100), None);

        cache.write(0, 100, data);
        let data = cache.read(0, 100);
        assert_eq!(data, [1; 4096]);

        cache.erase(0, 0);
        let data = cache.read(0, 100);
        assert_eq!(data, [0; 4096]);
    }
}