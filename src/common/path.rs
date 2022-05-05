use crate::inode::inode;
use crate::inode::inode_manager;
use crate::common::directory;

// Copy the next element from path into name.
// Return (path, name).
// If no name to remove, return None.
// The caller can check path == "" to se if the name is the last one.
pub fn skip_elem(path: String) -> Option<(String, String)> {
    let path = path.as_str();
    let mut index = 0;
    let temp_index;
    let len;
    for c in path.chars() {
        if c != '/' {
            break;
        }
        index += 1;
    }
    if index == path.len() {
        return None;
    }
    temp_index = index;
    while path[index..index+1] != '/'.to_string() {
        index += 1;
        if index == path.len() {
            break;
        }
    }
    len = index - temp_index;
    for c in path[index..].chars() {
        if c != '/' {
            break;
        }
        index += 1;
    }
    Some((path[index..].to_string(), path[temp_index..temp_index+len].to_string()))
}

// Look up and return the inode for a path name.
pub fn name_x(i_manager: &mut inode_manager::InodeManager, path: String, name: &mut String, name_i_parent: bool) -> Option<inode_manager::InodeLink> {
    let path = &mut path.clone();
    let mut ip;
    let mut next;
    if path.len() == 0 {
        return None;
    }
    if path[0..1] == '/'.to_string() {
        ip = i_manager.i_get(1).unwrap();
    } else {
        return None;
    }
    loop {
        let res = skip_elem(path.clone());
        if res.is_none() {
            (*path, *name) = ("".to_string(), "".to_string());
            break;
        }
        (*path, *name) = res.unwrap();
        if ip.borrow().file_type != inode::InodeFileType::Directory {
            return None;
        }
        if name_i_parent && path == "" {
            return Some(ip);
        }
        let res = directory::dir_lookup(&ip, name.clone());
        if res.is_none() {
            return None;
        }
        next = i_manager.i_get(res.unwrap().0).unwrap();
        ip = next;
    }
    if name_i_parent {
        return None;
    }
    return Some(ip);
}

pub fn name_i(i_manager: &mut inode_manager::InodeManager, path: String) -> Option<inode_manager::InodeLink> {
    let mut name = "".to_string();
    name_x(i_manager, path, &mut name, false)
}

pub fn name_i_parent(i_manager: &mut inode_manager::InodeManager, path: String, name: &mut String) -> Option<inode_manager::InodeLink> {
    name_x(i_manager, path, name, true)
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_skip_elem() {
        let res1 = skip_elem("a/bb/c".to_string());
        let res2 = skip_elem("///a//bb".to_string());
        let res3 = skip_elem("a".to_string());
        let res4 = skip_elem("".to_string());
        let res5 = skip_elem("////".to_string());
        assert_eq!(("bb/c".to_string(), "a".to_string()), res1.unwrap());
        assert_eq!(("bb".to_string(), "a".to_string()), res2.unwrap());
        assert_eq!(("".to_string(), "a".to_string()), res3.unwrap());
        assert_eq!(None, res4);
        assert_eq!(None, res5);
    }

    #[test]
    fn test_name_x() {
        let mut inode_manager = inode_manager::InodeManager::new();
        inode_manager.core_manager.borrow_mut().mount();
        let mut link = inode_manager.i_alloc();
        inode_manager.i_alloc();
        inode_manager.i_alloc();
        inode_manager.i_alloc();
        inode_manager.i_alloc();
        inode_manager.i_alloc();
        inode_manager.i_alloc();
        inode_manager.i_alloc();
        let stat = inode::InodeStat {
            file_type: inode::InodeFileType::Directory,
            ino: link.as_ref().unwrap().borrow().ino,
            size: 0,
            uid: 100,
            gid: 44,
            ref_cnt: 0,
            n_link: 1,
            mode: 0,
            last_accessed: (0, 0),
            last_modified: (0, 0),
            last_metadata_changed: (0, 0),
        };
        link.as_ref().unwrap().borrow_mut().modify_stat(stat);
        directory::dir_link(link.as_mut().unwrap(), 2, "test1.txt".to_string());
        directory::dir_link(link.as_mut().unwrap(), 3, "home".to_string());
        directory::dir_link(link.as_mut().unwrap(), 4, "test3.txt".to_string());
        let mut link = inode_manager.i_get(3);
        let stat = inode::InodeStat {
            file_type: inode::InodeFileType::Directory,
            ino: link.as_ref().unwrap().borrow().ino,
            size: 0,
            uid: 100,
            gid: 44,
            ref_cnt: 0,
            n_link: 1,
            mode: 0,
            last_accessed: (0, 0),
            last_modified: (0, 0),
            last_metadata_changed: (0, 0),
        };
        link.as_ref().unwrap().borrow_mut().modify_stat(stat);
        directory::dir_link(link.as_mut().unwrap(), 5, "test4.txt".to_string());
        directory::dir_link(link.as_mut().unwrap(), 6, "a.rs".to_string());
        directory::dir_link(link.as_mut().unwrap(), 7, "test5.txt".to_string());
        directory::dir_link(link.as_mut().unwrap(), 8, "test6.txt".to_string());
        let mut name = "".to_string();
        let link = name_x(&mut inode_manager, "/home/a.rs".to_string(), &mut name, false);
        assert_eq!(name, "".to_string());
        assert_eq!(link.as_ref().unwrap().borrow().ino, 6);
        let link = name_x(&mut inode_manager, "/home/a.rs".to_string(), &mut name, true);
        assert_eq!(name, "a.rs".to_string());
        assert_eq!(link.as_ref().unwrap().borrow().ino, 3);
    }
}