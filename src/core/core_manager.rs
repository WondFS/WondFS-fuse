use std::sync::Mutex;
use crate::buf;
use crate::core::bit;
use crate::core::pit;
use crate::core::vam;
use crate::util::array;
use crate::inode::inode;
use crate::inode::inode_event;
use crate::kv::fake_kv;
use crate::kv::raw_inode;
use crate::gc::gc_manager;
use crate::gc::gc_event;
use crate::gc::gc_manager::PageUsedStatus;

pub struct CoreManager {
    bit: bit::BIT,
    pit: pit::PIT,
    vam: vam::VAM,
    kv: fake_kv::FakeKV,
    gc: gc_manager::GCManager,
    buf_cache: buf::BufCache,
}

impl CoreManager {
    pub fn new() -> CoreManager {
        CoreManager {
            bit: bit::BIT::new(),
            pit: pit::PIT::new(),
            vam: vam::VAM::new(),
            kv: fake_kv::FakeKV::new(),
            gc: gc_manager::GCManager::new(),
            buf_cache: buf::BufCache::new(),
        }
    }

    pub fn read_sb(&mut self) {
        todo!()
    }

    pub fn mount(&mut self) {
        self.read_bit();
        self.read_pit();
    }
}

// KV Module
impl CoreManager {
    pub fn allocate_inode(&mut self) -> inode::Inode {
        let raw_inode = self.kv.allocate_inode();
        CoreManager::transfer_raw_inode_to_inode(&raw_inode)
    }

    pub fn get_inode(&mut self, ino: u32) -> inode::Inode {
        let mut raw_inode = self.kv.get_inode(ino);
        for entry in raw_inode.data.iter_mut() {
            let address = entry.address;
            entry.address = self.vam.get_available_address(entry.size);
            for i in 0..entry.size {
                self.vam.insert_map(address+i, entry.address+i);
            }
        }
        CoreManager::transfer_raw_inode_to_inode(&raw_inode)
    }

    pub fn update_inode(&mut self, inode: inode::Inode) {
        let mut inode = inode;
        for entry in inode.data.iter_mut() {
            entry.address = self.vam.get_physic_address(entry.address).unwrap()
        }
        let raw_inode = CoreManager::transfer_inode_to_raw_inode(&inode);
        self.kv.update_inode(raw_inode);
    }

    pub fn delete_inode(&mut self, ino: u32) {
        self.kv.delete_inode(ino);
    }

    pub fn get_raw_inode(&mut self, ino: u32) -> raw_inode::RawInode {
        self.kv.get_inode(ino)
    }

    pub fn update_raw_inode(&mut self, raw_inode: raw_inode::RawInode) {
        self.kv.update_inode(raw_inode);
    }
}

// GC Module
impl CoreManager {
    pub fn find_next_pos_to_write(&mut self, size: u32) -> u32 {
        let mut res;
        loop {
            res = self.gc.find_next_pos_to_write(size);
            if res.is_some() {
                break;
            }
            self.forward_gc();
        }
        res.unwrap()
    }

    pub fn forward_gc(&mut self) {
        let gc_group = self.gc.generate_gc_event();
        self.dispose_gc_group(gc_group);
    }

    pub fn background_gc(&mut self) {
        let flag = false;
        loop {
            if flag {
                let gc_group = self.gc.generate_gc_event();
                self.dispose_gc_group(gc_group);
            }
        }
    }

    pub fn set_main_table_page(&mut self, address: u32, status: PageUsedStatus) {
        self.gc.set_table(address, status);
    }

    pub fn dispose_gc_group(&mut self, gc_group: gc_event::GCEventGroup) {
        let mut gc_group = gc_group;
        CoreManager::sort_gc_event(&mut gc_group);
        for event in gc_group.events {
            match event {
                gc_event::GCEvent::Erase(event) => {
                    self.bit_begin_op();
                    self.pit_begin_op();
                    let start_index = event.block_no * 128;
                    let end_index = (event.block_no + 1) * 128;
                    for i in start_index..end_index {
                        self.update_bit(i, false);
                        self.clean_pit(i);
                    }
                    self.bit.end_op();
                    self.pit.end_op();
                    self.erase_block(event.block_no, true);
                }
                gc_event::GCEvent::Move(event) => {
                    let o_address = event.o_address;
                    let d_address = event.d_address;
                    let size = event.size;
                    let ino = event.ino;
                    let mut data = vec![];
                    for i in o_address..o_address + size {
                        data.push(self.read_page(i, true));
                        let v_address = self.vam.get_virtual_address(i);
                        if v_address.is_some() {
                            self.vam.update_map(d_address + i, v_address.unwrap());
                        }
                        self.dirty_pit(i);
                    }
                    for i in d_address..d_address + size {
                        self.update_bit(i, true);
                        self.update_pit(i, ino);
                        self.write_page(i, data[(i - d_address) as usize], true);
                    }
                    let mut raw_inode = self.get_raw_inode(ino);
                    for entry in raw_inode.data.iter_mut() {
                        if entry.address == o_address {
                            entry.address = d_address;
                            break;
                        }
                    }
                    self.update_raw_inode(raw_inode);
                }
                _ => ()
            }
        }
    }
}

// 管理BIT Region
impl CoreManager {
    pub fn read_bit(&mut self) {
        let mut data_1 = self.read_block(1, false);
        let data_2 = self.read_block(2, false);
        let mut flag = false;
        for page in data_2.iter() {
            for byte in page.iter() {
                if byte & 0b1111_1111 != 0 {
                    flag = true;
                    break;
                }
            }
        }
        if flag {
            self.erase_block(1, false);
            self.write_block(1, data_2.dup(), false);
            self.erase_block(2, false);
            data_1 = data_2;
        }
        self.set_bit(CoreManager::truncate_array_1_to_array_2(data_1));
    }

    pub fn set_bit(&mut self, data: array::Array2::<u8>) {
        for (i, byte) in data.iter().enumerate() {
            let mut byte = byte;
            for k in 0..8 {
                let index = i * 8 + k;
                if byte & 1 == 1 {
                    self.bit.init_page(index as u32, true);
                    self.set_main_table_page(index as u32, PageUsedStatus::Dirty);
                } else {
                    self.bit.init_page(index as u32, false);
                    self.set_main_table_page(index as u32, PageUsedStatus::Clean);
                }
                byte = byte >> 1;
            }
        }
    }

    pub fn update_bit(&mut self, address: u32, status: bool) {
        self.bit.set_page(address, status);
        match status {
            true => self.set_main_table_page(address, PageUsedStatus::Dirty),
            false => self.set_main_table_page(address, PageUsedStatus::Clean),
        }
        self.sync_bit();
    }

    pub fn sync_bit(&mut self) {
        if self.bit.need_sync() {
            let data = self.bit.encode();
            let data = CoreManager::truncate_array_2_to_array_1(data);
            self.write_block(2, data.dup(), false);
            self.erase_block(1, false);
            self.write_block(1, data, false);
            self.erase_block(2, false);
            self.bit.sync();
        }
    }

    pub fn bit_begin_op(&mut self) {
        self.bit.begin_op();
    }

    pub fn bit_end_op(&mut self) {
        self.bit.end_op();
        self.sync_bit();
    }
}

// 管理PIT Region
impl CoreManager {
    // 因为会爆栈，暂时改成1个Block
    pub fn read_pit(&mut self) {
        let mut data_1 = self.read_block(3, false);
        let data_2 = self.read_block(4, false);
        let mut flag = false;
        for page in data_2.iter() {
            for byte in page.iter() {
                if byte & 0b1111_1111 != 0 {
                    flag = true;
                    break;
                }
            }
        }
        if flag {
            self.erase_block(1, false);
            self.write_block(1, data_2.dup(), false);
            self.erase_block(2, false);
            data_1 = data_2;
        }
        self.set_pit(CoreManager::truncate_array_1_to_array_2(data_1))
    }

    pub fn set_pit(&mut self, data: array::Array2::<u8>) {
        let iter = pit::DataRegion::new(&data);
        for (index, ino) in iter.enumerate() {
            if ino != 0 {
                self.pit.init_page(index as u32, ino);
                self.set_main_table_page(index as u32, PageUsedStatus::Busy(ino));
            }
        }
    }

    pub fn update_pit(&mut self, address: u32, status: u32) {
        self.pit.set_page(address, status);
        self.set_main_table_page(address, PageUsedStatus::Busy(status));
        self.sync_pit();
    }

    pub fn dirty_pit(&mut self, address: u32) {
        self.pit.delete_page(address);
        self.set_main_table_page(address, PageUsedStatus::Dirty);
        self.sync_pit();
    }

    pub fn clean_pit(&mut self, address: u32) {
        self.pit.clean_page(address);
        self.set_main_table_page(address, PageUsedStatus::Clean);
        self.sync_pit();
    }
    
    pub fn sync_pit(&mut self) {
        if self.pit.need_sync() {
            let data = self.pit.encode();
            let data = CoreManager::truncate_array_2_to_array_1(data);
            self.write_block(4, data.dup(), false);
            self.erase_block(3, false);
            self.write_block(3, data, false);
            self.erase_block(4, false);
            self.pit.sync();
        }
    }

    pub fn pit_begin_op(&mut self) {
        self.pit.begin_op();
    }

    pub fn pit_end_op(&mut self) {
        self.pit.end_op();
        self.sync_pit();
    }
}

// 调用下层的接口，对上不可见
impl CoreManager {
    pub fn read_page(&mut self, address: u32, is_main: bool) -> [u8; 4096] {
        if is_main {
            self.buf_cache.read(0, address + 5 * 128)
        } else {
            self.buf_cache.read(0, address)
        }
    }

    pub fn read_block(&mut self, block_no: u32, is_main: bool) -> array::Array1::<[u8; 4096]> {
        let max_address = (block_no + 1) * 128;
        let mut address = max_address - 128;
        let mut block = vec![];
        while address < max_address {
            let page = self.read_page(address, is_main);
            address += 1;
            block.push(page);
        }
        let mut data = array::Array1::<[u8; 4096]>::new(128);
        data.init([0; 4096]);
        for (index, page) in block.into_iter().enumerate() {
            data.set(index as u32, page);
        }
        data
    }

    pub fn write_page(&mut self, address: u32, data: [u8; 4096], is_main: bool) {
        if is_main  {
            self.buf_cache.write(0, address + 5 * 128, data);
        } else {
            self.buf_cache.write(0, address, data);
        }
    }

    pub fn write_block(&mut self, block_no: u32, data: array::Array1::<[u8; 4096]>, is_main: bool) {
        if data.len() != 128 {
            panic!("CoreManager: write block not matched size");
        }
        let mut address = block_no * 128;
        for (index, data) in data.iter().enumerate() {
            self.write_page(address + index as u32, data, is_main);
        }
    }

    pub fn erase_block(&mut self, block_no: u32, is_main: bool) {
        if is_main {
            self.buf_cache.erase(0, block_no + 5);
        } else {
            self.buf_cache.erase(0, block_no);
        }
    }
}

// 对上层提供的读写接口
impl CoreManager {
    pub fn read_data(&mut self, v_address: u32) -> [u8; 4096] {
        let address = self.vam.get_physic_address(v_address).unwrap();
        self.read_page(address, true)
    }

    pub fn dispose_event_group(&mut self, event_group: inode_event::InodeEventGroup) -> Option<inode::Inode> {
        let mut inode = event_group.dup().inode;
        let mut event_group = event_group;
        CoreManager::sort_inode_event(&mut event_group);
        event_group.debug();
        if event_group.need_delete {
            for entry in inode.data.iter() {
                let address = self.vam.get_physic_address(entry.address).unwrap();
                for i in 0..entry.size {
                    self.dirty_pit(address + i);
                    let v_address = self.vam.get_virtual_address(address + i).unwrap();
                    self.vam.delete_map(address, v_address);
                }
            }
            self.kv.delete_inode(inode.ino);
            None
        } else {
            for entry in inode.data.iter_mut() {
                if entry.valid == false {
                    entry.valid = true;
                }
            }
            for event in event_group.events {
                match event {
                    inode_event::InodeEvent::AddContent(event) => {
                        let mut address = self.find_next_pos_to_write(event.size);
                        let mut v_address = self.vam.get_available_address(event.size);
                        let entry = inode::InodeEntry {
                            offset: event.offset,
                            len: event.len,
                            size: event.size,
                            valid: true,
                            address: v_address,
                        };
                        for i in 0..event.size {
                            let mut page = [0; 4096];
                            for j in 0..4096 {
                                let byte = event.content.get((i * 4096 + j) as usize);
                                if byte.is_some() {
                                    page[j as usize] = *byte.unwrap();
                                } else {
                                    page[j as usize] = 0;
                                }
                            }
                            self.write_page(address, page, true);
                            self.update_bit(address, true);
                            self.update_pit(address, inode.ino);
                            self.vam.insert_map(address, v_address);
                            address += 1;
                            v_address += 1;
                        }
                        inode.data.insert(event.index as usize, entry);

                    }
                    inode_event::InodeEvent::TruncateContent(event) => {
                        let mut entry = inode.data.get_mut(event.index as usize).unwrap();
                        entry.len = event.len;
                        entry.size = event.size;
                        entry.offset = event.offset;
                        let address = self.vam.get_physic_address(event.v_address).unwrap();
                        for i in event.size..event.o_size {
                            self.dirty_pit(address + i);
                            let v_address = self.vam.get_virtual_address(address + i).unwrap();
                            self.vam.delete_map(address, v_address);
                        }

                    }
                    inode_event::InodeEvent::ChangeContent(event) => {
                        let mut entry = inode.data.get_mut(event.index as usize).unwrap();
                        entry.offset = event.offset;
                        entry.address = event.v_address;
                    }
                    inode_event::InodeEvent::DeleteContent(event) => {
                        let mut entry = inode.data.get_mut(event.index as usize).unwrap();
                        let address = self.vam.get_physic_address(event.v_address).unwrap();
                        for i in 0..event.size {
                            self.dirty_pit(address + i);
                            let v_address = self.vam.get_virtual_address(address + i).unwrap();
                            self.vam.delete_map(address, v_address);
                        }
                        entry.valid = false;
                    }
                    inode_event::InodeEvent::ModifyStat(event) => {
                        inode.file_type = event.file_type;
                        inode.ino = event.ino;
                        inode.size = event.size;
                        inode.uid = event.uid;
                        inode.gid = event.gid;
                        inode.n_link = event.n_link;
                    }
                    _ => ()
                }
            }
            let mut remove_indexs = vec![];
            for index in 0..inode.data.len() {
                if !inode.data.get(index).unwrap().valid {
                    remove_indexs.push(index);
                }
            }
            for index in remove_indexs.into_iter().rev() {
                inode.data.remove(index);
            }
            let mut size = 0;
            for entry in inode.data.iter() {
                size += entry.len;
            }
            inode.size = size;
            let mut raw_inode = CoreManager::transfer_inode_to_raw_inode(&inode);
            for entry in raw_inode.data.iter_mut() {
                entry.address = self.vam.get_physic_address(entry.address).unwrap();
            }
            self.kv.update_inode(raw_inode);
            Some(inode)
        }
    }
}

impl CoreManager {
    // 注意这里不进行虚拟地址的转换
    pub fn transfer_raw_inode_to_inode(raw_inode: &raw_inode::RawInode) -> inode::Inode {
        let file_type;
        let mut data = vec![];
        match raw_inode.file_type {
            0 => file_type = inode::InodeFileType::File,
            1 => file_type = inode::InodeFileType::Directory,
            _ => panic!("CoreManager: transfer raw inode not available file type"),
        }
        for entry in raw_inode.data.iter() {
            let entry = inode::InodeEntry {
                len: entry.len,
                size: entry.size,
                offset: entry.offset,
                address: entry.address,
                valid: true,
            };
            data.push(entry);
        }
        inode::Inode {
            ino: raw_inode.ino,
            size: raw_inode.size,
            uid: raw_inode.uid,
            gid: raw_inode.gid,
            ref_cnt: raw_inode.ref_cnt,
            n_link: raw_inode.n_link,
            lock: Mutex::new(false),
            core: None,
            file_type,
            data,
            mode: todo!(),
            last_accessed: todo!(),
            last_modified: todo!(),
            last_metadata_changed: todo!(),
        }
    }
    
    // 注意这里不进行虚拟地址的转换
    pub fn transfer_inode_to_raw_inode(inode: &inode::Inode) -> raw_inode::RawInode {
        let file_type;
        let mut data = vec![];
        match inode.file_type {
            inode::InodeFileType::File => file_type = 0,
            inode::InodeFileType::Directory => file_type = 1,
        }
        for entry in inode.data.iter() {
            let entry = raw_inode::RawEntry {
                len: entry.len,
                size: entry.size,
                offset: entry.offset,
                address: entry.address,
            };
            data.push(entry);
        }
        raw_inode::RawInode {
            ino: inode.ino,
            uid: inode.uid,
            gid: inode.gid,
            size: inode.size,
            n_link: inode.n_link,
            ref_cnt: inode.ref_cnt,
            file_type,
            data,
            mode: inode.mode,
            last_accessed: inode.last_accessed,
            last_modified: inode.last_modified,
            last_metadata_changed: inode.last_metadata_changed,
        }
    }

    pub fn sort_gc_event(event_group: &mut gc_event::GCEventGroup) {
        let len = event_group.events.len();
        for i in 0..len {
            for j in 0..len - 1 - i {
                let index_1 =  event_group.events[j].get_index();
                let index_2 = event_group.events[j+1].get_index();
                if index_1 > index_2 {
                    let temp = event_group.events[j];
                    event_group.events[j] = event_group.events[j+1];
                    event_group.events[j+1] = temp;
                }
            }
        }
    }

    pub fn sort_inode_event(event_group: &mut inode_event::InodeEventGroup) {
        let len = event_group.events.len();
        for i in 0..len {
            for j in 0..len - 1 - i {
                let index_1 =  event_group.events[j].get_index();
                let index_2 = event_group.events[j+1].get_index();
                if index_1 > index_2 || index_1 == -1 {
                    let temp = event_group.events[j].clone();
                    event_group.events[j] = event_group.events[j+1].clone();
                    event_group.events[j+1] = temp;
                }
            }
        }
    }

    pub fn truncate_array_1_to_array_2(array: array::Array1<[u8; 4096]>) -> array::Array2::<u8> {
        if array.len() != 128 {
            panic!("CoreManager: truncate array1 to array2 not matched size");
        }
        let mut res = array::Array2::<u8>::new(128, 4096);
        res.init(0);
        for (i, page) in array.iter().enumerate() {
            for (j, byte) in page.into_iter().enumerate() {
                res.set(i as u32, j as u32, byte);
            }
        }
        res
    }

    pub fn truncate_array_2_to_array_1(array: array::Array2<u8>) -> array::Array1::<[u8; 4096]> {
        if array.len() != 128 * 4096 {
            panic!("CoreManager: truncate array2 to array1 not matched size");
        }
        let mut res = array::Array1::<[u8; 4096]>::new(128);
        res.init([0; 4096]);
        for (i, byte) in array.iter().enumerate() {
            let row = i / 4096;
            let column = i % 4096;
            let mut temp = res.get(row as u32);
            temp[column] = byte;
            res.set(row as u32, temp);
        }
        res
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn init_test() -> CoreManager {
        let mut manager = CoreManager::new();
        manager.mount();
        manager
    }

    #[test]
    fn basics() {

    }

    #[test]
    fn bit() {
        let mut manager = init_test();
        manager.update_bit(100, true);
        manager.update_bit(200, true);
        assert_eq!(manager.bit.need_sync(), false);
        let block1 = manager.read_block(1, false);
        let block2 = manager.read_block(2, false);
        for page in block2.iter() {
            for byte in page.iter() {
                assert_eq!(*byte, 0);
            }
        }
        assert_eq!(block1.get(0)[12]>>4 & 1, 1);
        assert_eq!(block1.get(0)[25] & 1, 1);
    }

    #[test]
    fn pit() {
        let mut manager = init_test();
        manager.update_pit(100, 67);
        manager.update_pit(200, 223);
        assert_eq!(manager.pit.need_sync(), false);
        let block1 = manager.read_block(3, false);
        let block2 = manager.read_block(4, false);
        for page in block2.iter() {
            for byte in page.iter() {
                assert_eq!(*byte, 0);
            }
        }
        let byte1 = (block1.get(0)[400] as u32) << 24;
        let byte2 = (block1.get(0)[401] as u32) << 16;
        let byte3 = (block1.get(0)[402] as u32) << 8;
        let byte4 = block1.get(0)[403] as u32;
        let temp = byte1 + byte2 + byte3 + byte4;
        assert_eq!(temp, 67);
        let byte1 = (block1.get(0)[800] as u32) << 24;
        let byte2 = (block1.get(0)[801] as u32) << 16;
        let byte3 = (block1.get(0)[802] as u32) << 8;
        let byte4 = block1.get(0)[803] as u32;
        let temp = byte1 + byte2 + byte3 + byte4;
        assert_eq!(temp, 223);
        manager.update_pit(1024, 2349);
        let block = manager.read_block(3, false);
        let byte1 = (block.get(1)[0] as u32) << 24;
        let byte2 = (block.get(1)[1] as u32) << 16;
        let byte3 = (block.get(1)[2] as u32) << 8;
        let byte4 = block.get(1)[3] as u32;
        let temp = byte1 + byte2 + byte3 + byte4;
        assert_eq!(temp, 2349);
        manager.dirty_pit(1024);
        let block = manager.read_block(3, false);
        let byte1 = (block.get(1)[0] as u32) << 24;
        let byte2 = (block.get(1)[1] as u32) << 16;
        let byte3 = (block.get(1)[2] as u32) << 8;
        let byte4 = block.get(1)[3] as u32;
        let temp = byte1 + byte2 + byte3 + byte4;
        assert_eq!(temp, 0);
        manager.clean_pit(200);
        let block = manager.read_block(3, false);
        let byte1 = (block.get(0)[800] as u32) << 24;
        let byte2 = (block.get(0)[801] as u32) << 16;
        let byte3 = (block.get(0)[802] as u32) << 8;
        let byte4 = block.get(0)[803] as u32;
        let temp = byte1 + byte2 + byte3 + byte4;
        assert_eq!(temp, 0);
    }

    #[test]
    fn kv() {
        let mut manager = init_test();
        let _ = manager.allocate_inode();
        let _ = manager.allocate_inode();
        let mut inode = manager.allocate_inode();
        inode.n_link = 3;
        manager.update_inode(inode);
        let inode = manager.get_inode(3);
        assert_eq!(inode.n_link, 3);
        manager.delete_inode(3);
        let mut raw_inode = manager.get_raw_inode(2);
        raw_inode.n_link = 100;
        manager.update_raw_inode(raw_inode);
        let inode = manager.get_inode(2);
        assert_eq!(inode.n_link, 100);
    }

    #[test]
    fn gc() {
        let mut manager = init_test();
        assert_eq!(manager.find_next_pos_to_write(10), 0);
        manager.update_bit(0, true);
        manager.update_pit(0, 1);
        manager.update_bit(1, true);
        manager.update_pit(1, 1);
        assert_eq!(manager.find_next_pos_to_write(10), 2);
        assert_eq!(manager.gc.find_next_pos_to_write_except(10, 0).unwrap(), 128);
        let gc_group = manager.gc.generate_gc_event();
        assert_eq!(gc_group.events[0], gc_event::GCEvent::Move(gc_event::MoveGCEvent{ index: 0, ino: 1, size: 2, o_address: 0, d_address: 128 }));
        assert_eq!(gc_group.events[1], gc_event::GCEvent::Erase(gc_event::EraseGCEvent{ index: 1, block_no: 0 }));
        manager.allocate_inode();
        manager.forward_gc();
    }

    #[test]
    fn underlay() {
        let mut manager = init_test();
        let data_1 = [1; 4096];
        let data_2 = [134; 4096];
        manager.write_page(100, data_1, false);
        manager.write_page(100, data_2, true);
        assert_eq!(manager.read_page(100, false), data_1);
        assert_eq!(manager.read_page(100, true), data_2);
        let mut data_3 = array::Array1::new(128);
        data_3.init([0; 4096]);
        data_3.set(100, [45; 4096]);
        manager.write_block(10, data_3, true);
        let mut data_3 = array::Array1::new(128);
        data_3.init([0; 4096]);
        data_3.set(100, [45; 4096]);
        manager.write_block(2, data_3, false);
        let mut data_3 = array::Array1::new(128);
        data_3.init([0; 4096]);
        data_3.set(100, [45; 4096]);
        assert_eq!(manager.read_block(10, true), data_3);
        assert_eq!(manager.read_block(2, false), data_3);
        manager.erase_block(0, false);
        assert_eq!(manager.read_page(100, false), [0; 4096]);
    }

    #[test]
    fn inode() {
        
    }

    #[test]
    fn util() {
        let mut array_1 = array::Array1::<[u8; 4096]>::new(128);
        array_1.init([0; 4096]);
        array_1.set(10, [10; 4096]);
        array_1.set(64, [11; 4096]);
        let mut array_2 = array::Array2::<u8>::new(128, 4096);
        array_2.init(0);
        for i in 0..4096 {
            array_2.set(10, i, 10);
            array_2.set(64, i, 11);
        }
        assert_eq!(CoreManager::truncate_array_1_to_array_2(array_1), array_2);
        let mut array_1 = array::Array1::<[u8; 4096]>::new(128);
        array_1.init([0; 4096]);
        array_1.set(10, [10; 4096]);
        array_1.set(64, [11; 4096]);
        assert_eq!(CoreManager::truncate_array_2_to_array_1(array_2), array_1);

        let raw_inode = raw_inode::RawInode {
            ino: 10,
            uid: 1,
            gid: 2,
            size: 100,
            n_link: 2,
            ref_cnt: 3,
            file_type: 1,
            data: vec![],
            mode: 0,
            last_accessed: (0, 0),
            last_modified: (0, 0),
            last_metadata_changed: (0, 0),
        };
        let inode = CoreManager::transfer_raw_inode_to_inode(&raw_inode);
        assert_eq!(inode.file_type, inode::InodeFileType::Directory);
        
        let mut inode = inode::Inode::new();
        inode.ino = 12;
        inode.file_type = inode::InodeFileType::File;
        let raw_inode = CoreManager::transfer_inode_to_raw_inode(&inode);
        assert_eq!(raw_inode.ino, 12);
        assert_eq!(raw_inode.file_type, 0);

        let mut gc_group = gc_event::GCEventGroup::new();
        let event1 = gc_event::EraseGCEvent { index: 2, block_no: 10 };
        let event2 = gc_event::EraseGCEvent { index: 0, block_no: 10 };
        let event3 = gc_event::MoveGCEvent{ index: 1, ino: 1, size: 2, o_address: 0, d_address: 128 };
        gc_group.events.push(gc_event::GCEvent::Erase(event1));
        gc_group.events.push(gc_event::GCEvent::Erase(event2));
        gc_group.events.push(gc_event::GCEvent::Move(event3));
        CoreManager::sort_gc_event(&mut gc_group);
        assert_eq!(gc_group.events[0], gc_event::GCEvent::Erase(event2));
        assert_eq!(gc_group.events[1], gc_event::GCEvent::Move(event3));
        assert_eq!(gc_group.events[2], gc_event::GCEvent::Erase(event1));
    }
}