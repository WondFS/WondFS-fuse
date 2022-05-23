//
// Disk Layer
//

use crate::driver::{disk, fake_disk};

// Disk Layer Main Controller Structure
pub struct DiskManager {
    pub is_virtual: bool,
    pub driver: Option<disk::DiskDriver>,
    pub fake_disk: Option<fake_disk::FakeDisk>,
}

// Disk Layer Simple Interface Function
impl DiskManager {
    pub fn new(is_virtual: bool) -> DiskManager {
        let mut driver = None;
        let mut fake_disk = None;
        if is_virtual {
            let block_num = 32;
            trace!("DiskManager: init fake disk with block num: {}", block_num);
            fake_disk = Some(fake_disk::FakeDisk::new(block_num * 128));
        } else {
            driver = Some(disk::DiskDriver::new());
        }
        DiskManager {
            is_virtual,
            driver,
            fake_disk,
        }
    }
}

// Disk Layer Main Interface Function
impl DiskManager {
    /// Read block from disk
    /// params:
    /// block_no - read block's block nunmber
    /// return:
    /// block data
    pub fn disk_read(&self, block_no: u32) -> [[u8; 4096]; 128] {
        trace!("DiskManager: read block block_no: {}", block_no);
        if self.is_virtual {
            return self.fake_disk.as_ref().unwrap().fake_disk_read(block_no);
        }
        self.driver.as_ref().unwrap().disk_read(block_no)
    }
    
    /// Write page to disk
    /// params:
    /// address - write page's address
    /// data - write data
    /// return:
    /// ()
    pub fn disk_write(&mut self, address: u32, data: [u8; 4096]) {
        trace!("DiskManager: write page adderss: {} ", address);
        if self.is_virtual {
            return self.fake_disk.as_mut().unwrap().fake_disk_write(address, data);
        }
        self.driver.as_mut().unwrap().disk_write(address, data);
    }
    
    /// Erase block in disk
    /// params:
    /// block_no - erase block's block number
    /// return:
    /// ()
    pub fn disk_erase(&mut self, block_no: u32) {
        trace!("DiskManager: erase block block_no: {}", block_no);
        if self.is_virtual {
            return self.fake_disk.as_mut().unwrap().fake_disk_erase(block_no);
        }
        self.driver.as_mut().unwrap().disk_erase(block_no);
    }
}

// Disk Layer Module Test
mod test {
    use super::*;

    #[test]
    fn basics() {
        let mut manager = DiskManager::new(true);

        let data = [1; 4096];
        manager.disk_write(100, data);

        let data = manager.disk_read(0);
        assert_eq!(data[100], [1; 4096]);

        let data =[2; 4096];
        manager.disk_write(256, data);
        let data = manager.disk_read(1);
        assert_eq!(data[2], [0; 4096]);

        manager.disk_erase(2);
        let data = manager.disk_read(1);
        assert_eq!(data[0], [0; 4096]);
    }
}