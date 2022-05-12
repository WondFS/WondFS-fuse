use crate::compress::compress;

pub struct Snappy {

}

impl Snappy {
    pub fn new() -> Self {
        Self {

        }
    }
}

impl compress::Compress for Snappy {
    fn encode(&self, bytes: &[u8]) -> Vec<u8> {
        use snap::write;
        use std::io::Write;

        let mut wtr = write::FrameEncoder::new(vec![]);
        wtr.write_all(bytes).unwrap();
        wtr.into_inner().unwrap()
    }
    
    fn decode(&self, bytes: &[u8]) -> Vec<u8> {
        use snap::read;
        use std::io::Read;
    
        let mut buf = vec![];
        read::FrameDecoder::new(bytes).read_to_end(&mut buf).unwrap();
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
        println!("{}", data.len());
        let compress = Snappy::new();
        let compressed = compress.encode(&data);
        println!("{}", compressed.len());
        let data = compress.decode(&compressed);
        println!("{}", data.len());
    }
}