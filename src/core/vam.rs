//
// VAM Manager
//

use std::collections::HashMap;

// VAM Manager Main Structure
pub struct VAM {
    count: u32,
    physical_address_table: HashMap<u32, u32>, // physical -> virtual
    virtual_address_table: HashMap<u32, u32>,  // virtual -> physical
}

// VAM Manager Simple Interface Function
impl VAM {
    pub fn new() -> VAM {
        VAM {
            count: 0,
            physical_address_table: HashMap::default(),
            virtual_address_table: HashMap::default(),
        }
    }

    pub fn get_available_address(&mut self, size: u32) -> u32 {
        let res = self.count;
        self.count += size;
        res
    }

    pub fn get_virtual_address(&self, address: u32) -> Option<u32> {
        let address = self.physical_address_table.get(&address);
        match address {
            Some(address) => {
                Some(address.clone())
            }
            None => None
        }
    }

    pub fn get_physic_address(&self, v_address: u32) -> Option<u32> {
        let address = self.virtual_address_table.get(&v_address);
        match address {
            Some(address) => {
                Some(address.clone())
            }
            None => None
        }
    }

    pub fn insert_map(&mut self, address: u32, v_address: u32) {
        // if self.physical_address_table.contains_key(&address) {
            // panic!("VAM: insert map has exist");
        // }
        self.physical_address_table.insert(address, v_address);
        self.virtual_address_table.insert(v_address, address);
    }

    pub fn update_map(&mut self, address: u32, v_address: u32) {
        let o_address = self.get_physic_address(v_address);
        if o_address.is_none() {
            panic!("VAM: update no that map");
        }
        self.delete_map(o_address.unwrap(), v_address);
        self.insert_map(address, v_address);
    }

    pub fn delete_map(&mut self, address: u32, v_address: u32) {
        if !self.physical_address_table.contains_key(&address) {
            panic!("VAM: delete no that map");
        }
        self.physical_address_table.remove(&address);
        self.virtual_address_table.remove(&v_address);
    }
}

// VAM Manager Moudle Test
#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    fn basics() {
        let mut vam = VAM::new();

        assert_eq!(vam.get_available_address(10), 0);
        assert_eq!(vam.get_available_address(10), 10);

        for i in 0..10 {
            vam.insert_map(i, 10 + i);
        }
        assert_eq!(vam.get_physic_address(14).unwrap(), 4);
        assert_eq!(vam.get_virtual_address(2).unwrap(), 12);

        vam.update_map(100, 13);
        assert_eq!(vam.get_physic_address(13).unwrap(), 100);
        assert_eq!(vam.get_virtual_address(100).unwrap(), 13);

        vam.delete_map(100, 13);
        assert_eq!(vam.get_physic_address(13), None);
        assert_eq!(vam.get_virtual_address(100), None);
    }
}