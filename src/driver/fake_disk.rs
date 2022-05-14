// Disk I/O Simulator

pub struct FakeDisk {
    pub size: u32,
    pub block_num: u32,
    pub data: Vec<[u8; 4096]>,
}

impl FakeDisk {
    pub fn new(size: u32) -> FakeDisk {
        let mut data = vec![];
        if size % 128 != 0 {
            panic!("FakeDisk: not available size")
        }
        for _ in 0..size {
            data.push([0; 4096]);
        }
        let block_num = size / 128;
        FakeDisk {
            size,
            data,
            block_num,
        }
    }

    pub fn fake_disk_read(&self, block_no: u32) -> [[u8; 4096]; 128] {
        if block_no > self.block_num - 1 {
            panic!("FakeKV: read at too big block number");
        }
        let mut data = [[0; 4096]; 128];
        let start_index = block_no * 128;
        let end_index = (block_no + 1) * 128;
        for index in start_index..end_index {
            data[(index - start_index) as usize] = self.data[index as usize];
        }
        data
    }
    
    pub fn fake_disk_write(&mut self, address: u32, data: [u8; 4096]) {
        if address > self.size - 1 {
            panic!("FakeDisk: write at too big address");
        }
        let o_data = self.data[address as usize];
        if o_data != [0; 4096] {
            panic!("FakeDisk: write at not clean address");
        }
        self.data[address as usize] = data;
    }
    
    pub fn fake_disk_erase(&mut self, block_no: u32) {
        if block_no > self.block_num - 1 {
            panic!("FakeKV: erase at too big block number");
        }
        let start_index = block_no * 128;
        let end_index = (block_no + 1) * 128;
        for index in start_index..end_index {
            self.data[index as usize] = [0; 4096];
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basics() {
        // Create 4MB Block
        let mut disk = FakeDisk::new(1024);

        let data = [1; 4096];
        disk.fake_disk_write(100, data);
        
        let data = disk.fake_disk_read(0);
        assert_eq!(data[100], [1; 4096]);

        let data =[2; 4096];
        disk.fake_disk_write(256, data);
        let data = disk.fake_disk_read(1);
        assert_eq!(data[2], [0; 4096]);
        
        disk.fake_disk_erase(2);
        let data = disk.fake_disk_read(1);
        assert_eq!(data[0], [0; 4096]);

        assert_eq!(disk.block_num, 8);
        assert_eq!(disk.size, 1024);
    }
}