use crate::compress::huffman;
use crate::compress::snappy;

#[derive(Copy, Clone, PartialEq)]
pub enum CompressType {
    None,
    Huffman,
    Snappy,
}

pub trait Compress {
    fn decode(&self, bytes: &[u8]) -> Vec<u8>;
    fn encode(&self, bytes: &[u8]) -> Vec<u8>;
}

// Stateless Compress Manager
pub struct CompressManager;

impl CompressManager {
    pub fn encode(bytes: &[u8]) -> (Vec<u8>, CompressType) {
        let snappy = snappy::Snappy::new();
        let huffman = huffman::HuffmanCodec::new();
        let res = snappy.encode(bytes);
        (res, CompressType::Snappy)
    }

    pub fn encode_with_type(bytes: &[u8], compress_type: CompressType) -> Vec<u8> {
        match compress_type {
            CompressType::None => bytes.to_owned(),
            CompressType::Huffman => {
                let huffman = huffman::HuffmanCodec::new();
                huffman.encode(bytes)
            },
            CompressType::Snappy => {
                let snappy = snappy::Snappy::new();
                snappy.encode(bytes)
            },
        }
    }

    pub fn decode(bytes: &[u8], compress_type: CompressType) -> Vec<u8> {
        let snappy = snappy::Snappy::new();
        let huffman = huffman::HuffmanCodec::new();
        snappy.decode(bytes)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basics() {
        let data = "fsfjlahuhdwnf.v.sljp;jdqdsjdfhalkshdlhliqjdna,dnlawjdla.jdj.lskd.wnkak".as_bytes();
        let ret = CompressManager::encode(&data);
        CompressManager::decode(&ret.0, ret.1);
    }
}