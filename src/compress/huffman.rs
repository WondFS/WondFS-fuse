use crate::compress::compress;
use std::cell::RefCell;
use std::collections::{HashMap, hash_map::Iter};
use std::fmt::Display;
use std::rc::Rc;
use std::str;
use std::time::SystemTime;

type RefHuffmanTree = Rc<RefCell<HuffmanTree>>;
type Weight = u32;

pub struct HuffmanTree {
    pub value: Option<char>,
    pub weight: Weight,
    pub parent: Option<RefHuffmanTree>,
    pub left: Option<RefHuffmanTree>,
    pub right: Option<RefHuffmanTree>,
}

impl HuffmanTree {
    pub fn new() -> Self {
        Self {
            value: None,
            weight: 0,
            parent: None,
            left: None,
            right: None,
        }
    }

    pub fn build(char_weight: CharWeightMap) -> RefHuffmanTree {
        let n = char_weight.len();
        let total = 2 * n - 1;
        let vec = (0..total)
            .map(|_| Rc::new(RefCell::new(Self::new())))
            .collect::<Vec<Rc<RefCell<HuffmanTree>>>>();
        char_weight.iter()
            .enumerate()
            .into_iter()
            .for_each(|(index, (ch, weight))| {
                vec[index].borrow_mut().value = Some(*ch);
                vec[index].borrow_mut().weight = *weight;
            });
        for index in n..total {
            let m1 = Self::find_min(&vec[..index]).unwrap();
            m1.borrow_mut().parent = Some(vec[index].clone());
            let m2 = Self::find_min(&vec[..index]).unwrap();
            m2.borrow_mut().parent = Some(vec[index].clone());
            let w1 = m1.as_ref().borrow().weight;
            let w2 = m2.as_ref().borrow().weight;
            let weight = w1 + w2;
            vec[index].borrow_mut().weight = weight;
            vec[index].borrow_mut().left = Some(m1.clone());
            vec[index].borrow_mut().right = Some(m2.clone());
        }
        vec.last().unwrap().clone()
    }

    fn find_min(tree_slice: &[Rc<RefCell<HuffmanTree>>]) -> Option<Rc<RefCell<HuffmanTree>>> {
        let mut min = Weight::MAX;
        let mut result = None;
        for tree in tree_slice {
            let tree_cell = tree.as_ref();
            if tree_cell.borrow().parent.is_none() && tree_cell.borrow().weight < min {
                min = tree_cell.borrow().weight;
                result = Some(tree.clone());
            }
        }
        result
    }
}

pub struct CharWeightMap {
    pub inner: HashMap<char, Weight>
}

impl CharWeightMap {
    pub fn build() -> Self {
        let mut map = HashMap::new();
        for c in 0..=255 as u8 {
            map.insert(c as char, 1);
        }
        let weights = ['e', 'E', 'a', 'A', 'i', 'I', 'r', 'R', 't', 'T', 'o', 'O', 'n', 'N', 's', 
                                   'S', 'l', 'L', 'c', 'C', 'u', 'U', 'p', 'P', 'm', 'M', 'd', 'D', 'h', 'H',
                                   'g', 'G', 'b', 'B', 'y', 'Y', 'f', 'F', 'v', 'V', 'w', 'W', 'k', 'K', 'x',
                                   'X', 'z', 'Z', 'q', 'Q', 'j', 'J'];
        for (index, c) in weights.iter().rev().enumerate() {
            *map.get_mut(&c).unwrap() = 2 + index as u32;
        }
        Self { inner: map }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter(&self) -> Iter<char, Weight> {
        self.inner.iter()  
    }
}

pub struct HuffmanBinaryMap {
    pub inner: HashMap<char, Vec<bool>>
}

impl HuffmanBinaryMap {
    pub fn build(huffman_tree: RefHuffmanTree) -> Self {
        let mut map = HashMap::new();
        Self::tree_dfs(&Some(huffman_tree), &mut map, &mut vec![]);
        Self { inner: map }
    }
    fn tree_dfs(
        tree: &Option<RefHuffmanTree>, 
        map: &mut HashMap<char, Vec<bool>>,
        vec: &mut Vec<bool>
    ) {
        if let Some(tree) = tree {
            let tree = tree.as_ref().borrow();
            if let Some(ch) = tree.value {
                map.insert(ch, vec.clone());
            }
            vec.push(false);
            Self::tree_dfs(&tree.left, map, vec);
            let last = vec.last_mut().unwrap();
            *last = true;
            Self::tree_dfs(&tree.right, map, vec);
            vec.pop();
        }
    }
}

impl Display for HuffmanBinaryMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = String::new();
        self.inner.iter()
            .for_each(|(c, vec)| {
                let mut bit_str = String::new();
                vec.iter().for_each(|b| {
                    bit_str += if *b { "1" } else { "0" }
                });
                buf += format!("{}:{}\n", *c as u32, bit_str).as_str();
            });
        f.write_str(buf.as_str())
    }
}

pub struct HuffmanCodec {
    pub coefficient: u8,
    pub efficiency: u128,
    pub bit_map: HuffmanBinaryMap,
    pub decode_map: DecodeConfig,
}

impl HuffmanCodec {
    pub fn new() -> Self {
        let weight_map = CharWeightMap::build();
        let tree = HuffmanTree::build(weight_map);
        let bit_map = HuffmanBinaryMap::build(tree);
        let decode_map = DecodeConfig::build(&format!("space:{}\ncapacity:{}\n{}", 0, 0, bit_map));
        Self {
            coefficient: 1,
            efficiency: 0,
            bit_map,
            decode_map,
        }
    }

    pub fn modify_coefficient(&mut self, coeffi: u8) {
        self.coefficient = (6 * coeffi as u16 / 10 + 4 * self.coefficient as u16 / 10) as u8;
    }

    pub fn modify_efficiency(&mut self, effi: u128) {
        self.efficiency = 6 * effi / 10 + 4 * self.efficiency / 10;
    }
}

impl compress::Compress for HuffmanCodec {
    fn decode(&mut self, bytes: &[u8]) -> Vec<u8> {
        let source = bytes.to_vec();
        let start_time = SystemTime::now();
        let mut result = String::with_capacity(self.decode_map.capacity);
        let bit_str = source.iter()
            .map(|num| {
                format!("{u8:>0width$b}", u8=num, width=8)
            })
            .collect::<Vec<String>>()
            .join("");
        let mut tmp_str = String::with_capacity(20);
        let last_idx = bit_str.len() - self.decode_map.space as usize;
        for (i, ch) in bit_str.char_indices() {
            if i >= last_idx {
                break;
            }
            tmp_str.push(ch);
            if let Some(mch) = self.decode_map.get(&tmp_str) {
                result.push(*mch);
                tmp_str.clear();
            }
        }
        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time).ok().unwrap();
        self.modify_efficiency(duration.as_micros());
        result.into_bytes()
    }

    fn encode(&mut self, bytes: &[u8]) -> Vec<u8> {        
        let source = std::str::from_utf8(bytes).unwrap().to_string();
        let start_time = SystemTime::now();
        let mut result: Vec<u8> = vec![];
        let (mut buf, mut count) = (0, 0);
        for (_, ch) in source.char_indices() {
            let vec = self.bit_map.inner.get(&ch).unwrap();
            vec.iter().for_each(|b| {
                buf <<= 1;
                if *b { buf |= 1 }
                count += 1;
                if count >= 8 {
                    result.push(buf);
                    buf = 0;
                    count = 0;
                }
            })
        }
        let mut space = 0u8;
        if count != 0 {
            space = 8 - count;
            buf <<= space;
            result.push(buf);
        }
        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time).ok().unwrap();
        self.modify_efficiency(duration.as_micros());
        self.modify_coefficient((result.len() * 100 / bytes.len()) as u8);
        result
    }
}

pub struct DecodeConfig {
    pub inner: HashMap<String, char>,
    pub space: u8,
    pub capacity: usize,
}
impl DecodeConfig {
    pub fn build(source: &String) -> Self {
        let mut map = HashMap::default();
        let (mut space, mut capacity) = (0u8, 0usize);
        let arr = source.split("\n");
        for s in arr {
            let pair: Vec<&str> = s.split(":").collect();
            if pair.len() != 2 {
                continue;
            }
            let (ch, bit) = (pair[0], pair[1]);
            match ch {
                "space" => {
                    space = u8::from_str_radix(bit, 10).unwrap();
                    continue;
                },
                "capacity" => {
                    capacity = usize::from_str_radix(bit, 10).unwrap();
                    continue;
                },
                _ => (),
            }
            map.insert(bit.to_owned(), char::from_u32(u32::from_str_radix(ch, 10).unwrap()).unwrap());
        };
        Self { inner: map, space, capacity }
    }
    pub fn get(&self, k: &String) -> Option<&char> {
        self.inner.get(k)
    }
}

#[cfg(test)]
mod test {
    use crate::compress::compress::Compress;
    use super::*;

    #[test]
    fn basics() {
        let data = "fsfjlahuhdwnf.v.sljp;jdqdsjdfhalkshdlhliqjfsfjlahuhdwnf.v.sljp;jdqdsjdfhalkshdlhliqjdna,dnlawjdla.jdj.lskd.wnkakadmbDmabdmadahqbdkfsfsknasnwnkdnsnsckwkcwjlkrjflqwjclamlqwdjwlfdjlamflcmljwijrlqflkmlkmlam;c;wk;rk;qkf;,l.e,s;lad;lca;skc;lkasc;k;wk;ekr;qkw;fk;qk;aclks;lck;kwe;qlkf;lwekf;lqk;kca/kcq/;kf;/wq;er/;wemc;kasd/vjlerhgnkv,bsfnqlnfknjk,env,nq,nfwqnf.wmlmvavqljwlejl   jdlj    llk jcljljhajsjqbwd bdkcdashlcahlcb,kbd,    n,kew   kdkqwn,cknc ,k,qnwn qbd,k   bx, mbmasbcmbambmdbamcbamscmnavfkjfhkqwhecquhakcbkwb,ek,fbqwfqwbfnqefkqfqewfqwfqvaddna,dnlawjdla.jdj.lskd.wnkakadmbDmabdmadahqbdkfsfsknasnwnkdnsnsckwkcwjlkrjflqwjclamlqwdjwlfdjlamflcmljwijrlqflkmlkmlam;c;wk;rk;qkf;,l.e,s;lad;lca;skc;lkasc;k;wk;ekr;qkw;fk;qk;aclks;lck;kwe;qlkf;lwekf;lqk;kca/kcq/;kf;/wq;er/;wemc;kasd/vjlerhgnkv,bsfnqlnfknjk,env,nq,nfwqnf.wmlmvavqljwlejl   jdlj    llk jcljljhajsjqbwd bdkcdashlcahlcb,kbd,    n,kew   kdkqwn,cknc ,k,qnwn qbd,k   bx, mbmasbcmbambmdbamcbamscmnavfkjfhkqwhecquhakcbkwb,ek,fbqwfqwbfnqefkqfqewfqwfqvadvavafsfjlahuhdwnf.v.sljp;jdqdsjdfhalkshdlhliqjdna,dnlawjdla.jdj.lskd.wnkakadmbDmabdmadahqbdkfsfsknasnwnkdnsnsckwkcwjlkrjflqwjclamlqwdjwlfdjlamflcmljwijrlqflkmlkmlam;c;wk;rk;qkf;,l.e,s;lad;lca;skc;lkasc;k;wk;ekr;qkw;fk;qk;aclks;lck;kwe;qlkf;lwekf;lqk;kca/kcq/;kf;/wq;er/;wemc;kasd/vjlerhgnkv,bsfnqlnfknjk,env,nq,nfwqnf.wmlmvavqljwlejl   jdlj    llk jcljljhajsjqbwd bdkcdashlcahlcb,kbd,    n,kew   kdkqwn,cknc ,k,qnwn qbd,k   bx, mbmasbcmbambmdbamcbamscmnavfkjfhkqwhecquhakcbkwb,ek,fbqwfqwbfnqefkqfqewfqwfqvadfsfjlahuhdwnf.v.sljp;jdqdsjdfhalkshdlhliqjdna,dnlawjdla.jdj.lskd.wnkakadmbDmabdmadahqbdkfsfsknasnwnkdnsnsckwkcwjlkrjflqwjclamlqwdjwlfdjlamflcmljwijrlqflkmlkmlam;c;wk;rk;qkf;,l.e,s;lad;lca;skc;lkasc;k;wk;ekr;qkw;fk;qk;aclks;lck;kwe;qlkf;lwekf;lqk;kca/kcq/;kf;/wq;er/;wemc;kasd/vjlerhgnkv,bsfnqlnfknjk,env,nq,nfwqnf.wmlmvavqljwlejl   jdlj    llk jcljljhajsjqbwd bdkcdashlcahlcb,kbd,    n,kew   kdkqwn,cknc ,k,qnwn qbd,k   bx, mbmasbcmbambmdbamcbamscmnavfkjfhkqwhecquhakcbkwb,ek,fbqwfqwbfnqefkqfqewfqwfqvadfsfjlahuhdwnf.v.sljp;jdqdsjdfhalkshdlhliqjdna,dnlawjdla.jdj.lskd.wnkakadmbDmabdmadahqbdkfsfsknasnwnkdnsnsckwkcwjlkrjflqwjclamlqwdjwlfdjlamflcmljwijrlqflkmlkmlam;c;wk;rk;qkf;,l.e,s;lad;lca;skc;lkasc;k;wk;ekr;qkw;fk;qk;aclks;lck;kwe;qlkf;lwekf;lqk;kca/kcq/;kf;/wq;er/;wemc;kasd/vjlerhgnkv,bsfnqlnfknjk,env,nq,nfwqnf.wmlmvavqljwlejl   jdlj    llk jcljljhajsjqbwd bdkcdashlcahlcb,kbd,    n,kew   kdkqwn,cknc ,k,qnwn qbd,k   bx, mbmasbcmbambmdbamcbamscmnavfkjfhkqwhecquhakcbkwb,ek,fbqwfqwbfnqefkqfqewfqwfqvadfqfqv".as_bytes();
        let mut compress = HuffmanCodec::new();
        let compressed = compress.encode(&data);
        compress.decode(&compressed);
    }
}