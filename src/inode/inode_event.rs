use crate::inode::inode;

pub struct InodeEventGroup {
    pub inode: inode::Inode,
    pub need_delete: bool,
    pub events: Vec<InodeEvent>,
}

impl InodeEventGroup {
    pub fn new() -> InodeEventGroup {
        InodeEventGroup {
            inode: inode::Inode::new(),
            need_delete: false,
            events: vec![],
        }
    }

    pub fn dup(&self) -> InodeEventGroup {
        InodeEventGroup {
            inode: self.inode.copy_inode(),
            need_delete: self.need_delete,
            events: self.events.clone(),
        }
    }

    pub fn debug(&self) {
        // println!("InodeEventGroup::Debug:{} {}, {}", self.inode.ino, self.need_delete, self.events.len());
        // for (index, event) in self.events.iter().enumerate() {
        //     match event {
        //         InodeEvent::AddContent(event) => {
        //             println!("InodeEventGroup::Debug:{}, Add index: {} offset: {} len: {} size: {} content: {:?}", index, event.index, event.offset, event.len, event.size, event.content);
        //         },
        //         InodeEvent::TruncateContent(event) => {
        //             println!("InodeEventGroup::Debug:{}, Truncate index: {} offset: {} len: {} size: {} o_size: {} v_address: {}", index, event.index, event.offset, event.len, event.size, event.o_size, event.v_address);
        //         },
        //         InodeEvent::ChangeContent(event) => {
        //             println!("InodeEventGroup::Debug:{}, Change index: {} offset: {} v_address: {}", index, event.index, event.offset, event.v_address);
        //         },
        //         InodeEvent::DeleteContent(event) => {
        //             println!("InodeEventGroup::Debug:{}, Delete index: {} size: {} v_address: {}", index, event.index, event.size, event.v_address);
        //         },
        //         InodeEvent::ModifyStat(event) => {
        //             println!("InodeEventGroup::Debug:{}, Modify size: {} uid: {} gid: {} n_link: {}", index, event.size, event.uid, event.gid, event.n_link);
        //         },
        //         InodeEvent::None => {
        //             println!("InodeEventGroup::Debug:{}, None", index);
        //         },
        //     }
        // }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum InodeEvent {
    AddContent(AddContentInodeEvent),
    TruncateContent(TruncateContentInodeEvent),
    ChangeContent(ChangeContentInodeEvent),
    DeleteContent(DeleteContentInodeEvent),
    ModifyStat(ModifyInodeStatInodeEvent),
    None,
}

impl InodeEvent {
    pub fn get_index(&self) -> i32 {
        let mut index = 0;
        match self {
            InodeEvent::AddContent(event) => index = event.index as i32,
            InodeEvent::TruncateContent(event) => index = event.index as i32,
            InodeEvent::ChangeContent(event) => index = event.index as i32,
            InodeEvent::DeleteContent(event) => index = event.index as i32,
            InodeEvent::ModifyStat(_) => index = -1,
            InodeEvent::None => (),
        }
        index
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct AddContentInodeEvent {
    pub index: u32,
    pub offset: u32,
    pub len: u32,
    pub size: u32,
    pub content: Vec<u8>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TruncateContentInodeEvent {
    pub index: u32,
    pub offset: u32,
    pub len: u32,
    pub size: u32,
    pub o_size: u32,
    pub v_address: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ChangeContentInodeEvent {
    pub index: u32,
    pub offset: u32,
    pub v_address: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct DeleteContentInodeEvent {
    pub index: u32,
    pub size: u32,
    pub v_address: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ModifyInodeStatInodeEvent {
    pub file_type: inode::InodeFileType,
    pub ino: u32,
    pub size: u32,
    pub uid: u32,
    pub gid: u32,
    pub n_link: u8,
}