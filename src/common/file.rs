// use crate::inode::inode;
// use crate::inode::inode::InodeStat;
// use crate::inode::inode_manager;

// #[derive(PartialEq)]
// pub enum FileDescriptorType {
//     NONE,
//     PIPE,
//     INODE,
//     DEVICE,
// }

// pub enum FileType {
//     TDIR,    // Directory
//     TFILE,   // File
//     TDEVICE, // Device
// }

// pub struct FileStat {
//     pub dev: u8,
//     pub ino: u32,
//     pub file_type: FileType,
//     pub n_link: u8,
//     pub size: u32,
// }

// pub struct File {
//     pub off: u32,
//     pub ref_cnt: u8,
//     pub read_able: u8,
//     pub writeable: u8,
//     pub fd_type: FileDescriptorType,
//     pub inode: Option<inode_manager::InodeLink>,
// }

// impl File {
//     // Get metadata about file f.
//     pub fn file_stat(&self) -> Option<FileStat> {
//         if self.fd_type == FileDescriptorType::INODE || self.fd_type == FileDescriptorType::DEVICE {
//             let inode_stat = self.inode.as_ref().unwrap().borrow().get_stat();
//             return Some(File::transfer_inode_stat_to_stat(inode_stat));
//         }
//         None
//     }

//     // Read from file f.
//     pub fn file_read(&mut self, len: u32, buf: &mut Vec<u8>) -> i32 {
//         buf.clear();
//         let mut count = 0;
//         if self.read_able == 0 {
//             return -1;
//         }
//         if self.fd_type == FileDescriptorType::INODE {
//             count = self.inode.as_ref().unwrap().borrow_mut().read(self.off, len, buf);
//             if count > 0 {
//                 self.off += count as u32;
//             }
//         }
//         count
//     }

//     // Write to file f.
//     pub fn file_write(&mut self, len: u32, buf: &Vec<u8>) -> i32 {
//         let mut ret = 0;
//         if self.writeable == 0 {
//             return -1;
//         }
//         if self.fd_type == FileDescriptorType::INODE {
//             let res = self.inode.as_ref().unwrap().borrow_mut().write(self.off, len, &buf);
//             if res {
//                 self.off += len;
//                 ret = len as i32;
//             } else {
//                 ret = -1;
//             }
//         }
//         ret
//     }
// }

// impl File {
//     pub fn new() -> File {
//         File {
//             off: 0,
//             ref_cnt: 0,
//             read_able: 0,
//             writeable: 0,
//             fd_type: FileDescriptorType::NONE,
//             inode: None,
//         }
//     }

//     pub fn transfer_inode_stat_to_stat(inode_stat: inode::InodeStat) -> FileStat {
//         FileStat {
//             dev: 0,
//             ino: inode_stat.ino,
//             file_type: FileType::TFILE,
//             n_link: inode_stat.n_link,
//             size: inode_stat.size,
//         }
//     }

//     pub fn transfer_stat_to_inode(stat: FileStat) -> inode::InodeStat {
//         InodeStat {
//             file_type: inode::InodeFileType::File,
//             ino: stat.ino,
//             size: stat.size,
//             uid: 0,
//             gid: 0,
//             ref_cnt: 0,
//             n_link: stat.n_link,
//         }
//     }
// }