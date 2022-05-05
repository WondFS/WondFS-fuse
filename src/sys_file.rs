// // //
// // // File-system system calls.
// // //

// use std::sync::Arc;
// use crate::fake_proc;
// use crate::inode::inode;
// use crate::common::path;
// use crate::common::directory;
// use crate::common::file_table;
// use crate::inode::inode_manager;

// // fetch system call argument as a file descriptor
// // and return both the descriptor and the corresoinding struct file.
// pub fn arg_fd() -> Option<(u32, file_table::FileLink)> {
//     let fd = 0;
//     let proc = fake_proc::my_proc();
//     let mut file = None;
//     if fd < 0 || fd >= proc.max_file || proc.file[fd as usize].is_none()  {
//         return None;
//     }
//     file = Some(Arc::clone(&proc.file[fd as usize].as_ref().unwrap()));
//     Some((fd as u32, file.unwrap()))
// }

// // Allocate a file descriptor for the given file.
// pub fn fdalloc(file: file_table::FileLink) -> i32 {
//     let mut proc = fake_proc::my_proc();
//     for fd in 0..proc.max_file {
//         if proc.file[fd as usize].is_none() {
//             proc.file[fd as usize] = Some(file);
//             return fd as i32;
//         }
//     }
//     -1
// }

// pub fn sys_dup() -> i32 {
//     let mut proc = fake_proc::my_proc();
//     let (_, file) = arg_fd().unwrap();
//     let fd = fdalloc(file);
//     if fd < 0 {
//         return -1;
//     }
//     proc.file_table.file_dup(&proc.file[fd as usize].as_ref().unwrap());
//     fd
// }

// pub fn sys_read() -> i32 {
//     let (_, file) = arg_fd().unwrap();
//     let len = 0;
//     let mut buf = vec![];
//     // ？ 为啥直接返回函数结果会报错捏
//     let ret = file.borrow_mut().file_read(len, &mut buf);
//     ret
// }

// pub fn sys_write() -> i32 {
//     let (_, file) = arg_fd().unwrap();
//     let len = 0;
//     let buf = vec![];
//     let ret = file.borrow_mut().file_write(len, &buf);
//     ret
// }

// pub fn sys_close() -> i32 {
//     let mut proc = fake_proc::my_proc();
//     let (fd, file) = arg_fd().unwrap();
//     proc.file[fd as usize] = None;
//     proc.file_table.file_close(file);
//     0
// }

// pub fn sys_fstat() -> i32 {
//     let (_, file) = arg_fd().unwrap();
//     let _ = file.borrow().file_stat().unwrap();
//     0
// }

// // Create the path new as a link to the same inode as old.
// pub fn sys_link() -> i32 {
//     let mut proc = fake_proc::my_proc();
//     let mut name = "".to_string();
//     let new = "".to_string();
//     let old = "".to_string();
//     let ip = path::name_i(&mut proc.inode_manager, old);
//     if ip.is_none() {
//         return -1;
//     }
//     let ip = ip.unwrap();
//     if ip.borrow().file_type == inode::InodeFileType::Directory {
//         return -1;
//     }
//     let mut stat = ip.borrow_mut().get_stat();
//     stat.n_link += 1;
//     ip.borrow_mut().modify_stat(stat);
//     let dp = path::name_i_parent(&mut proc.inode_manager, new, &mut name);
//     if dp.is_none() {
//         let mut stat = ip.borrow_mut().get_stat();
//         stat.n_link -= 1;
//         ip.borrow_mut().modify_stat(stat);
//         return -1;
//     }
//     let dp = dp.unwrap();
//     let ret = directory::dir_link(&mut proc.inode_manager, dp, ip.borrow().ino, name);
//     if !ret {
//         return -1;
//     }
//     0
// }

// // Is the directory dp empty except for "." and ".." ?
// pub fn is_dir_empty(inode: inode_manager::InodeLink) -> bool {
//     let mut off = 14;
//     while off < inode.borrow().size {
//         let mut buf = vec![];
//         if inode.borrow_mut().read(off, 14, &mut buf) != 14 {
//             panic!();
//         }
//         let entry = directory::DirectoryParser::decode(&buf).unwrap();
//         if entry.ino != 0 {
//             return false;
//         }
//         off += 14;
//     }
//     true
// }

// pub fn sys_unlink() -> i32 {
//     let mut proc = fake_proc::my_proc();
//     let mut name = "".to_string();
//     let path = "".to_string();
//     let dp = path::name_i_parent(&mut proc.inode_manager, path, &mut name);
//     if dp.is_none() {
//         return -1;
//     }
//     let dp = dp.unwrap();
//     if name == "." || name == ".." {
//         return -1;
//     }
//     let res = directory::dir_lookup(&mut proc.inode_manager, Arc::clone(&dp), name.clone());
//     if res.is_none() {
//         return -1;
//     }
//     let (ip, _) = res.unwrap();
//     if ip.borrow().n_link < 1 {
//         panic!();
//     }
//     if ip.borrow().file_type == inode::InodeFileType::Directory && !is_dir_empty(Arc::clone(&ip)) {
//         return -1;
//     }
//     let ret = directory::dir_unlink(&mut proc.inode_manager, Arc::clone(&dp), ip.borrow().ino, name.clone());
//     if !ret {
//         return -1;
//     }
//     if ip.borrow().file_type == inode::InodeFileType::Directory {
//         let mut stat = dp.borrow_mut().get_stat();
//         stat.n_link -= 1;
//         dp.borrow_mut().modify_stat(stat);
//     }
//     let mut stat = ip.borrow_mut().get_stat();
//     stat.n_link -= 1;
//     ip.borrow_mut().modify_stat(stat);
//     0
// }

// pub fn create(path: String, inode_type: inode::InodeFileType) -> Option<inode_manager::InodeLink> {
//     let mut proc = fake_proc::my_proc();
//     let mut name = "".to_string();
//     let dp = path::name_i_parent(&mut proc.inode_manager, path, &mut name);
//     if dp.is_none() {
//         return None;
//     }
//     let dp = dp.unwrap();
//     let res = directory::dir_lookup(&mut proc.inode_manager, Arc::clone(&dp), name.clone());
//     if res.is_some() {
//         let (ip, _) = res.unwrap();
//         if inode_type == inode::InodeFileType::File && ip.borrow().file_type == inode::InodeFileType::File {
//             return Some(ip);
//         }
//         return None;
//     }
//     let ip = proc.inode_manager.i_alloc();
//     if ip.is_none() {
//         return None;
//     }
//     let ip = ip.unwrap();
//     let mut stat = ip.borrow_mut().get_stat();
//     stat.file_type = inode_type;
//     stat.n_link = 1;
//     ip.borrow_mut().modify_stat(stat);
//     // Create . and .. entries.
//     if inode_type == inode::InodeFileType::Directory {
//         // for ".."
//         let mut stat = dp.borrow_mut().get_stat();
//         stat.file_type = inode_type;
//         stat.n_link += 1;
//         dp.borrow_mut().modify_stat(stat);
//         let res = directory::dir_link(&mut proc.inode_manager, Arc::clone(&ip), ip.borrow().ino, ".".to_string());
//         if !res {
//             panic!();
//         }
//         let res = directory::dir_link(&mut proc.inode_manager, Arc::clone(&ip), dp.borrow().ino, "..".to_string());
//         if !res {
//             panic!();
//         }
//     }
//     let res = directory::dir_link(&mut proc.inode_manager, Arc::clone(&dp), ip.borrow().ino, name);
//     if !res {
//         panic!()
//     }
//     return Some(ip);
// }

// pub fn sys_open() -> i32 {
//     todo!()
// }

// pub fn sys_mkdir() -> i32 {
//     let path = "".to_string();
//     let ip = create(path, inode::InodeFileType::Directory);
//     if ip.is_none() {
//         return -1;
//     }
//     return 0;
// }

// pub fn sys_chdir() -> i32 {
//     let mut proc = fake_proc::my_proc();
//     let path = "".to_string();
//     let ip = path::name_i(&mut proc.inode_manager, path);
//     if ip.is_none() {
//         return -1;
//     }
//     let ip = ip.unwrap();
//     if ip.borrow().file_type != inode::InodeFileType::Directory {
//         return -1;
//     }
//     proc.cwd = ip;
//     0
// }