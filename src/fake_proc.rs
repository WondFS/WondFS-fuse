// use std::cell::RefCell;
// use std::sync::Arc;
// use crate::inode::inode_manager;
// use crate::common::file_table;
// use crate::inode::inode;

// // Per-process state
// pub struct Proc {
//     pub file: Vec<Option<file_table::FileLink>>,
//     pub cwd: inode_manager::InodeLink,
//     pub max_file: u32,
//     pub file_table: file_table::FileTable,
//     pub inode_manager: inode_manager::InodeManager,
// }

// pub fn my_proc() -> Proc {
//     let mut file = vec![];
//     for _ in 0..30 {
//         file.push(None);
//     }
//     Proc {
//         file,
//         cwd: Arc::new(RefCell::new(inode::Inode::new())),
//         max_file: 16,
//         file_table: file_table::FileTable::new(),
//         inode_manager: inode_manager::InodeManager::new(),
//     }
// }