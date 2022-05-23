//
// Check Center
// 

use crate::util::crc32;
use crate::util::ecc;

// Check Center Implemented Encode/Decode Algorithms
#[derive(PartialEq, Debug)]
pub enum CheckType {
    Crc32, // 0x00 Crc32
    Ecc,   // 0x01 Ecc
}

// Check Center Structure
pub struct CheckCenter;

// Check Center Main Interface Function
impl CheckCenter {
    /// Check page
    /// params:
    /// data - page data
    /// sa - signature
    /// return:
    /// check result
    /// check type
    /// data after corrected if can be corrected
    pub fn check(data: &[u8; 4096], sa: &Vec<u8>) -> (bool, CheckType, Option<[u8; 4096]>) {
        if sa.len() != 128 {
            panic!("CheckCenter: check sa not valid size");
        }
        let check_type;
        if sa[127] == 0x00 {
            check_type = CheckType::Crc32;
        } else {
            check_type = CheckType::Ecc;
        }
        match check_type {
            CheckType::Crc32 => {
                let bytes = &sa[0..4];
                let byte_1 = (bytes[0] as u32) << 24;
                let byte_2 = (bytes[1] as u32) << 16;
                let byte_3 = (bytes[2] as u32) << 8;
                let byte_4 = bytes[3] as u32;
                let signature = byte_1 + byte_2 + byte_3 + byte_4;
                let ret = CheckCenter::check_crc_32(data, signature);
                (ret.0, CheckType::Crc32, ret.1)
            },
            CheckType::Ecc => {
                unimplemented!()
            },
        }
    }

    /// Sign page
    /// params:
    /// data - page data
    /// address - page's address
    /// sign_type - check type
    /// return:
    /// siganture
    pub fn sign(data: &[u8; 4096], address: u32, sign_type: CheckType) -> Vec<u8> {
        match sign_type {
            CheckType::Crc32 => {
                CheckCenter::sign_crc_32(data, address)
            },
            CheckType::Ecc => {
                unimplemented!()
            },
        }
    }

    /// Extract page's address in signature
    /// params:
    /// signature - signature
    /// return:
    /// page's address
    pub fn extract_address(signature: &Vec<u8>) -> u32 {
        if signature.len() != 128 {
            panic!("CheckCenter: extract address signature not valid size");
        }
        let bytes = &signature[123..127];
        let byte_1 = (bytes[0] as u32) << 24;
        let byte_2 = (bytes[1] as u32) << 16;
        let byte_3 = (bytes[2] as u32) << 8;
        let byte_4 = bytes[3] as u32;
        byte_1 + byte_2 + byte_3 + byte_4
    }
}

// Check Center Internal Function
impl CheckCenter {
    fn check_crc_32(data: &[u8; 4096], signature: u32) -> (bool, Option<[u8; 4096]>) {
        let crc = crc32::Crc::<u32>::new(&crc32::CRC_32_ISCSI);
        let check_sum = crc.checksum(data);
        (check_sum == signature, None)
    }

    fn sign_crc_32(data: &[u8; 4096], address: u32) -> Vec<u8> {
        let crc = crc32::Crc::<u32>::new(&crc32::CRC_32_ISCSI);
        let check_sum = crc.checksum(data);
        let mut sa = vec![];
        for _ in 0..128 {
            sa.push(0);
        }
        let byte_1 = (check_sum >> 24) as u8;
        let byte_2 = (check_sum >> 16) as u8;
        let byte_3 = (check_sum >> 8) as u8;
        let byte_4 = check_sum as u8;
        sa[0] = byte_1;
        sa[1] = byte_2;
        sa[2] = byte_3;
        sa[3] = byte_4;
        sa[119] = 0x33;
        sa[120] = 0x33;
        sa[121] = 0xaa;
        sa[122] = 0xaa;
        let byte_1 = (address >> 24) as u8;
        let byte_2 = (address >> 16) as u8;
        let byte_3 = (address >> 8) as u8;
        let byte_4 = address as u8;
        sa[123] = byte_1;
        sa[124] = byte_2;
        sa[125] = byte_3;
        sa[126] = byte_4;
        sa[127] = 0x00;
        sa
    }
}

// Check Center Module Test
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basics() {
        let mut data = [27; 4096];
        data[234] = 23;
        data[123] = 78;
        data[89] = 12;
        let sa = CheckCenter::sign(&data, 383, CheckType::Crc32);
        let ret = CheckCenter::check(&data, &sa);
        assert_eq!(ret.0, true);
        assert_eq!(ret.1, CheckType::Crc32);
        assert_eq!(ret.2, None);
        assert_eq!(CheckCenter::extract_address(&sa), 383);
    }
}