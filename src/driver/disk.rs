//
// Disk I/O
//

// Disk Driver Structure
pub struct DiskDriver;

// Disk Driver Simple Interface Function
impl DiskDriver {
    pub fn new() -> DiskDriver {
        DiskDriver {

        }
    }
}

// Disk Driver Main Interface Function
impl DiskDriver {
    /// Read block from disk
    /// params:
    /// block_no - read block's block nunmber
    /// return:
    /// block data
    pub fn disk_read(&self, block_no: u32) -> [[u8; 4096]; 128] {
        unimplemented!()
    }
    
    /// Write page to disk
    /// params:
    /// address - write page's address
    /// data - write data
    /// return:
    /// ()
    pub fn disk_write(&mut self, address: u32, data: [u8; 4096]) {
        unimplemented!()
    }
    
    /// Erase block in disk
    /// params:
    /// block_no - erase block's block number
    /// return:
    /// ()
    pub fn disk_erase(&mut self, block_no: u32) {
        unimplemented!()
    }
}