use crate::util::array;

const MAGIC_NUMBER_1: u32 = 0x2222ffff; // 标志坏块映射块
const MAGIC_NUMBER_2: u32 = 0x3333aaaa; // 标志校验存储块

// Reserved Region Block Type
pub enum BlockType {
    MappingTable, // 存储tlb/plb映射表
    Signature,    // 存储数据签名
    Used,         // 被用作坏块映射
    Unused,       // 未被使用的
    Unknown,      // 未知
}

/// Judge reserved region block's block type
/// param:
/// data - block data
/// return:
/// block type
pub fn judge_block_type(data: &array::Array1<[u8; 4096]>) -> BlockType {
    if data.get(0)[0] == 0x22 && data.get(0)[1] == 0x22 && data.get(0)[2] == 0xff && data.get(0)[3] == 0xff {
        return BlockType::MappingTable;
    }
    if data.get(0)[119] == 0x33 && data.get(0)[120] == 0x33 && data.get(0)[121] == 0xaa && data.get(0)[122] == 0xaa {
        return BlockType::Signature;
    }
    BlockType::Unknown
}

/// Convert [[u8; 4096]; 128] to array:Array1<[u8; 4096]>
/// param:
/// data: block data
/// return:
/// converted data
pub fn transfer(data: &[[u8; 4096]; 128]) -> array::Array1<[u8; 4096]> {
    let mut res = array::Array1::new(128);
    res.init([0; 4096]);
    for i in 0..128 {
        res.set(i, data[i as usize]);
    }
    res
}

/// Convert array:Array1<[u8; 4096]> to [[u8; 4096]; 128]
/// param:
/// data: data to be converted
/// return:
/// converted data
pub fn reverse(data: &array::Array1<[u8; 4096]>) -> [[u8; 4096]; 128] {
    let mut ret = [[0; 4096]; 128];
    for i in 0..128 {
        ret[i] = data.get(i as u32);
    }
    ret
}