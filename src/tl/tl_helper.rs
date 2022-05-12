use crate::util::array;

pub enum BlockType {
    MappingTable, // 存储plb/tlb映射表
    Signature,    // 存储数据签名
    Used,         // 被用作坏块映射
    Unused,       // 未被使用的
    Unknown,      // 未知
}

pub fn judge_block_type(data: &array::Array1<[u8; 4096]>) -> BlockType {
    if data.get(0)[0] == 0x22 && data.get(0)[1] == 0x22 && data.get(0)[2] == 0xff && data.get(0)[3] == 0xff {
        return BlockType::MappingTable;
    }
    if data.get(0)[0] == 0x33 && data.get(0)[1] == 0x33 && data.get(0)[2] == 0xaa && data.get(0)[3] == 0xaa {
        return BlockType::Signature;
    }
    BlockType::Unknown
}

pub fn transfer(data: [[u8; 4096]; 128]) -> array::Array1<[u8; 4096]> {
    let mut res = array::Array1::new(128);
    res.init([0; 4096]);
    for i in 0..128 {
        res.set(i, data[i as usize]);
    }
    res
}

pub fn reverse(data: &array::Array1<[u8; 4096]>) -> [[u8; 4096]; 128] {
    let mut ret = [[0; 4096]; 128];
    for i in 0..128 {
        ret[i] = data.get(i as u32);
    }
    ret
}

pub fn truncate_array_1_to_array_2(array: array::Array1<[u8; 4096]>) -> array::Array2::<u8> {
    if array.len() != 128 {
        panic!("truncate array1 to array2 not matched size");
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
        panic!("truncate array2 to array1 not matched size");
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