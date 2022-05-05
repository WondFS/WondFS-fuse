#[derive(Copy, Clone)]
pub struct RawEntry {
    pub len: u32,
    pub size: u32,
    pub offset: u32,
    pub address: u32,
}

pub struct RawInode {
    pub ino: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub n_link: u8,
    pub ref_cnt: u8,
    pub file_type: u8, // 0 File 1 Directory
    pub mode: u16,
    pub last_accessed: (i64, u32),
    pub last_modified: (i64, u32),
    pub last_metadata_changed: (i64, u32),
    pub data: Vec<RawEntry>,
}