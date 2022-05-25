//
// GC Manager
//

use crate::gc::gc_define::*;
use crate::gc::gc_event;
use crate::gc::block_table;
use crate::core::bit;

// GC Manager Structure
pub struct GCManager {
    need_sync: bool,  
    hot_blocks: Vec<u32>,
    normal_blocks: Vec<u32>,
    cold_blocks: Vec<u32>,
    block_table: block_table::BlockTable,
}

// GC Manager Simple Interface Function
impl GCManager {
    pub fn new() -> GCManager {
        GCManager {
            need_sync: true,
            hot_blocks: vec![],
            normal_blocks: vec![],
            cold_blocks: vec![],
            block_table: block_table::BlockTable::new(8),
        }
    }
}

// GC Manager Main Interface Function
impl GCManager {
    /// Find write address in disk for n page size data
    /// params:
    /// size - data size
    /// return:
    /// write address
    pub fn find_next_pos_to_write(&self, size: u32) -> Option<u32> {
        for block in self.block_table.table.iter() {
            if block.reserved_size >= size {
                let offset = block.reserved_offset;
                return Some((block.block_no * 128) as u32 + offset);
            }
        }
        None
    }

    /// Generate GC events based on the specified GCStrategy
    /// params:
    /// strategy - GC strategy
    /// return:
    /// GC events
    pub fn new_gc_event(&mut self, strategy: GCStrategy) -> gc_event::GCEventGroup {
        self.sync();
        let block_no = self.choose_gc_block(strategy);
        self.generate_gc_group(block_no)
    }

    /// Get page used status in block table
    /// params:
    /// address - page's address
    /// return:
    /// page used status
    pub fn get_page(&self, address: u32) -> PageUsedStatus {
        self.block_table.get_page(address)
    }

    /// Set page used status in block table
    /// params:
    /// address - page's address
    /// status - page used status
    /// return:
    /// ()
    pub fn set_page(&mut self, address: u32, status: PageUsedStatus) {
        self.block_table.set_page(address, status);
    }

    /// Get block information in block table
    /// params:
    /// block_no - block's block number
    /// return:
    /// block information
    pub fn get_block_info(&self, block_no: u32) -> &block_table::BlockInfo {
        self.block_table.get_block_info(block_no)
    }

    /// Set block information by BIT Segement in block table
    /// params:
    /// block_no - block's block number
    /// segment - BIT segment information
    /// return:
    /// ()
    pub fn set_block_info(&mut self, block_no: u32, segment: bit::BITSegement) {
        self.block_table.set_last_erase_time(block_no, segment.last_erase_time);
        self.block_table.set_erase_count(block_no, segment.erase_count);
        self.block_table.set_average_age(block_no, segment.average_age);
    }

    /// Erase block in block table
    /// params:
    /// block_no - block's block number
    /// return:
    /// ()
    pub fn erase_block(&mut self, block_no: u32) {
        self.block_table.erase_block(block_no);
    }


}

// GC Manager Internal Function
impl GCManager {
    fn generate_gc_group(&self, block_no: u32) -> gc_event::GCEventGroup {
        let mut used_entries: Vec<(u32, u32, u32, u32)> = vec![];
        let start_index = block_no * 128;
        let end_index = (block_no + 1) * 128;
        let mut size = 0;
        let mut last_entry: Option<(u32, u32, u32, u32)> = None;
        for address in start_index..end_index {
            let status = self.block_table.get_page(address);
            match status {
                PageUsedStatus::Busy(ino) => {
                    if last_entry.is_some() {
                        if last_entry.unwrap().0 == ino {
                            size += 1;
                        } else {
                            last_entry.as_mut().unwrap().1 = size;
                            used_entries.push(last_entry.unwrap());
                            last_entry = Some((ino, 0, address, 0));
                        }
                    } else {
                        last_entry = Some((ino, 0, address, 0));
                        size = 1;
                    }
                }
                _ => {
                    if last_entry.is_some() {
                        last_entry.as_mut().unwrap().1 = size;
                        used_entries.push(last_entry.unwrap());
                        last_entry = None;
                        size = 0;
                    }
                }
            }
        }
        if last_entry.is_some() {
            last_entry.as_mut().unwrap().1 = size;
            used_entries.push(last_entry.unwrap());
        }
        for entry in used_entries.iter_mut() {
            let d_address = self.find_next_pos_to_write_except(entry.1, block_no);
            entry.3 = d_address.unwrap();
        }
        let mut gc_group = gc_event::GCEventGroup::new();
        let mut index = 0;
        for entry in used_entries {
            let event = gc_event::MoveGCEvent {
                index,
                ino: entry.0,
                size: entry.1,
                o_address: entry.2,
                d_address: entry.3,
            };
            gc_group.events.push(gc_event::GCEvent::Move(event));
            index += 1;
        }
        let event = gc_event::EraseGCEvent {
            index,
            block_no,
        };
        gc_group.events.push(gc_event::GCEvent::Erase(event));
        gc_group
    }

    fn choose_gc_block(&self, strategy: GCStrategy) -> u32 {
        match strategy {
            GCStrategy::Forward => {
                let mut gc_block = self.block_table.get_block_info(0);
                for block in self.block_table.table.iter() {
                    if block.get_utilize_ratio() < gc_block.get_utilize_ratio() {
                        gc_block = block;
                    }
                }
                gc_block.block_no
            },
            GCStrategy::BackgroundSimple => {
                let mut gc_block = self.block_table.get_block_info(0);
                for block in self.block_table.table.iter() {
                    if block.get_utilize_ratio() < gc_block.get_utilize_ratio() {
                        gc_block = block;
                    }
                }
                gc_block.block_no
            },
            GCStrategy::BackgroundCold => {
                let mut gc_block = self.block_table.get_block_info(0);
                for block in self.block_table.table.iter() {
                    if block.get_utilize_ratio() < gc_block.get_utilize_ratio() {
                        gc_block = block;
                    }
                }
                gc_block.block_no
            },
        }
    }

    fn find_next_pos_to_write_except(&self, size: u32, block_no: u32) -> Option<u32> {
        for block in self.block_table.table.iter() {
            if block.reserved_size >= size && block.block_no != block_no {
                let offset = block.reserved_offset;
                return Some((block.block_no * 128) as u32 + offset);
            }
        }
        None
    }

    fn sync(&mut self) {
        if self.need_sync {
            for index in 0..self.block_table.size {
                let info = self.get_block_info(index);
                if info.average_age > COLDAGEKEY {
                    self.cold_blocks.push(index);
                    continue;
                }
                if info.average_age < HOTAGEKEY {
                    self.hot_blocks.push(index);
                    continue;
                }
                self.normal_blocks.push(index);
            }
            self.need_sync = false;
        }
    }

}


// GC Manager Module Test
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basics() {
        let mut manager = GCManager::new();

        assert_eq!(manager.find_next_pos_to_write(5), Some(0));
        
        manager.set_page(0, PageUsedStatus::Busy(0));
        manager.set_page(1, PageUsedStatus::Busy(0));
        manager.set_page(2, PageUsedStatus::Busy(0));
        manager.set_page(3, PageUsedStatus::Busy(0));
        manager.set_page(4, PageUsedStatus::Busy(0));

        assert_eq!(manager.get_page(0), PageUsedStatus::Busy(0));
        assert_eq!(manager.find_next_pos_to_write(128), Some(128));

        let event = manager.new_gc_event(GCStrategy::Forward);
        assert_eq!(event.events[0], gc_event::GCEvent::Move(gc_event::MoveGCEvent{ index: 0, ino: 0, size: 5, o_address: 0, d_address: 128 }));
        assert_eq!(event.events[1], gc_event::GCEvent::Erase(gc_event::EraseGCEvent{ index: 1, block_no: 0 }));
    }
}