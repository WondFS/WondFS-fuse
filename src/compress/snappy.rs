use crate::compress::compress;
use std::time::SystemTime;

pub struct Snappy {
    pub coefficient: u8,
    pub efficiency: u128,
}

impl Snappy {
    pub fn new() -> Self {
        Self {
            coefficient: 1,
            efficiency: 0,
        }
    }

    pub fn modify_coefficient(&mut self, coeffi: u8) {
        self.coefficient = (6 * coeffi as u16 / 10 + 4 * self.coefficient as u16 / 10) as u8;
    }

    pub fn modify_efficiency(&mut self, effi: u128) {
        self.efficiency = 6 * effi / 10 + 4 * self.efficiency / 10;
    }
}

impl compress::Compress for Snappy {
    fn encode(&mut self, bytes: &[u8]) -> Vec<u8> {
        use snap::write;
        use std::io::Write;
        let start_time = SystemTime::now();
        let mut wtr = write::FrameEncoder::new(vec![]);
        wtr.write_all(bytes).unwrap();
        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time).ok().unwrap();
        let res = wtr.into_inner().unwrap();
        self.modify_efficiency(duration.as_micros());
        self.modify_coefficient((res.len() * 100 / bytes.len()) as u8);
        res
    }
    
    fn decode(&mut self, bytes: &[u8]) -> Vec<u8> {
        use snap::read;
        use std::io::Read;
        let start_time = SystemTime::now();
        let mut buf = vec![];
        read::FrameDecoder::new(bytes).read_to_end(&mut buf).unwrap();
        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time).ok().unwrap();
        self.modify_efficiency(duration.as_micros());
        buf
    }
}

#[cfg(test)]
mod test {
    use crate::compress::compress::Compress;
    use super::*;
    
    #[test]
    fn basics() {
        let data = "fsfjlahuhdwnf.v.sljp;jdqdsjdfhalkshdlhliqjfsfjlahuhdwnf.v.sljp;jdqdsjdfhalkshdlhliqjdna,dnlawjdla.jdj.lskd.wnkakadmbDmabdmadahqbdkfsfsknasnwnkdnsnsckwkcwjlkrjflqwjclamlqwdjwlfdjlamflcmljwijrlqflkmlkmlam;c;wk;rk;qkf;,l.e,s;lad;lca;skc;lkasc;k;wk;ekr;qkw;fk;qk;aclks;lck;kwe;qlkf;lwekf;lqk;kca/kcq/;kf;/wq;er/;wemc;kasd/vjlerhgnkv,bsfnqlnfknjk,env,nq,nfwqnf.wmlmvavqljwlejl   jdlj    llk jcljljhajsjqbwd bdkcdashlcahlcb,kbd,    n,kew   kdkqwn,cknc ,k,qnwn qbd,k   bx, mbmasbcmbambmdbamcbamscmnavfkjfhkqwhecquhakcbkwb,ek,fbqwfqwbfnqefkqfqewfqwfqvaddna,dnlawjdla.jdj.lskd.wnkakadmbDmabdmadahqbdkfsfsknasnwnkdnsnsckwkcwjlkrjflqwjclamlqwdjwlfdjlamflcmljwijrlqflkmlkmlam;c;wk;rk;qkf;,l.e,s;lad;lca;skc;lkasc;k;wk;ekr;qkw;fk;qk;aclks;lck;kwe;qlkf;lwekf;lqk;kca/kcq/;kf;/wq;er/;wemc;kasd/vjlerhgnkv,bsfnqlnfknjk,env,nq,nfwqnf.wmlmvavqljwlejl   jdlj    llk jcljljhajsjqbwd bdkcdashlcahlcb,kbd,    n,kew   kdkqwn,cknc ,k,qnwn qbd,k   bx, mbmasbcmbambmdbamcbamscmnavfkjfhkqwhecquhakcbkwb,ek,fbqwfqwbfnqefkqfqewfqwfqvadvavafsfjlahuhdwnf.v.sljp;jdqdsjdfhalkshdlhliqjdna,dnlawjdla.jdj.lskd.wnkakadmbDmabdmadahqbdkfsfsknasnwnkdnsnsckwkcwjlkrjflqwjclamlqwdjwlfdjlamflcmljwijrlqflkmlkmlam;c;wk;rk;qkf;,l.e,s;lad;lca;skc;lkasc;k;wk;ekr;qkw;fk;qk;aclks;lck;kwe;qlkf;lwekf;lqk;kca/kcq/;kf;/wq;er/;wemc;kasd/vjlerhgnkv,bsfnqlnfknjk,env,nq,nfwqnf.wmlmvavqljwlejl   jdlj    llk jcljljhajsjqbwd bdkcdashlcahlcb,kbd,    n,kew   kdkqwn,cknc ,k,qnwn qbd,k   bx, mbmasbcmbambmdbamcbamscmnavfkjfhkqwhecquhakcbkwb,ek,fbqwfqwbfnqefkqfqewfqwfqvadfsfjlahuhdwnf.v.sljp;jdqdsjdfhalkshdlhliqjdna,dnlawjdla.jdj.lskd.wnkakadmbDmabdmadahqbdkfsfsknasnwnkdnsnsckwkcwjlkrjflqwjclamlqwdjwlfdjlamflcmljwijrlqflkmlkmlam;c;wk;rk;qkf;,l.e,s;lad;lca;skc;lkasc;k;wk;ekr;qkw;fk;qk;aclks;lck;kwe;qlkf;lwekf;lqk;kca/kcq/;kf;/wq;er/;wemc;kasd/vjlerhgnkv,bsfnqlnfknjk,env,nq,nfwqnf.wmlmvavqljwlejl   jdlj    llk jcljljhajsjqbwd bdkcdashlcahlcb,kbd,    n,kew   kdkqwn,cknc ,k,qnwn qbd,k   bx, mbmasbcmbambmdbamcbamscmnavfkjfhkqwhecquhakcbkwb,ek,fbqwfqwbfnqefkqfqewfqwfqvadfsfjlahuhdwnf.v.sljp;jdqdsjdfhalkshdlhliqjdna,dnlawjdla.jdj.lskd.wnkakadmbDmabdmadahqbdkfsfsknasnwnkdnsnsckwkcwjlkrjflqwjclamlqwdjwlfdjlamflcmljwijrlqflkmlkmlam;c;wk;rk;qkf;,l.e,s;lad;lca;skc;lkasc;k;wk;ekr;qkw;fk;qk;aclks;lck;kwe;qlkf;lwekf;lqk;kca/kcq/;kf;/wq;er/;wemc;kasd/vjlerhgnkv,bsfnqlnfknjk,env,nq,nfwqnf.wmlmvavqljwlejl   jdlj    llk jcljljhajsjqbwd bdkcdashlcahlcb,kbd,    n,kew   kdkqwn,cknc ,k,qnwn qbd,k   bx, mbmasbcmbambmdbamcbamscmnavfkjfhkqwhecquhakcbkwb,ek,fbqwfqwbfnqefkqfqewfqwfqvadfqfqv".as_bytes();
        let mut compress = Snappy::new();
        let compressed = compress.encode(&data);
        compress.decode(&compressed);
    }
}