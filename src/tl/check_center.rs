use crate::util::crc32;

#[derive(PartialEq, Debug)]
pub enum CheckType {
    Crc32, // 0x00
    Ecc,   // 0x01
}

pub struct CheckCenter;

impl CheckCenter {
    pub fn check(data: &[u8; 4096], sa: &[u8; 128]) -> (bool, CheckType, Option<[u8; 4096]>) {
        let check_type;
        if sa[127] == 0x00 {
            check_type = CheckType::Crc32;
        } else {
            check_type = CheckType::Ecc;
        }
        if check_type == CheckType::Crc32 {
            let bytes = &sa[0..4];
            let byte_1 = (bytes[0] as u32) << 24;
            let byte_2 = (bytes[1] as u32) << 16;
            let byte_3 = (bytes[2] as u32) << 8;
            let byte_4 = bytes[3] as u32;
            let signature = byte_1 + byte_2 + byte_3 + byte_4;
            let ret = CheckCenter::check_crc_32(data, signature);
            return (ret.0, CheckType::Crc32, ret.1);
        }
        (false, check_type, None)
    }

    pub fn check_crc_32(data: &[u8; 4096], signature: u32) -> (bool, Option<[u8; 4096]>) {
        let crc = crc32::Crc::<u32>::new(&crc32::CRC_32_ISCSI);
        let check_sum = crc.checksum(data);
        (check_sum == signature, None)
    }

    pub fn sign_crc_32(data: &[u8; 4096]) -> [u8; 128] {
        let crc = crc32::Crc::<u32>::new(&crc32::CRC_32_ISCSI);
        let check_sum = crc.checksum(data);
        let mut sa = [0; 128];
        let byte_1 = (check_sum >> 24) as u8;
        let byte_2 = (check_sum >> 16) as u8;
        let byte_3 = (check_sum >> 8) as u8;
        let byte_4 = check_sum as u8;
        sa[0] = byte_1;
        sa[1] = byte_2;
        sa[2] = byte_3;
        sa[3] = byte_4;
        sa[127] = 0x00;
        sa
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basics() {
        let mut data = [27; 4096];
        data[234] = 23;
        data[123] = 78;
        data[89] = 12;
        let sa = CheckCenter::sign_crc_32(&data);
        let ret = CheckCenter::check(&data, &sa);
        assert_eq!(ret.0, true);
        assert_eq!(ret.1, CheckType::Crc32);
        assert_eq!(ret.2, None);
    }
}