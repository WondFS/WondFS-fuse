// GC Event Group Structure
pub struct GCEventGroup {
    pub events: Vec<GCEvent>,
}

impl GCEventGroup {
    pub fn new() -> GCEventGroup {
        GCEventGroup {
            events: vec![],
        }
    }
}

// GC Event Structure
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum GCEvent {
    Erase(EraseGCEvent),
    Move(MoveGCEvent),
    None,
}

impl GCEvent {
    pub fn get_index(&self) -> u32 {
        let mut index = 0;
        match self {
            GCEvent::Erase(event) => index = event.index,
            GCEvent::Move(event) => index = event.index,
            GCEvent::None => ()
        }
        index
    }
}


// CC Erase Event Structure
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct EraseGCEvent {
    pub index: u32,
    pub block_no: u32,
}

// GC Move Event Structure
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct MoveGCEvent {
    pub index: u32,
    pub ino: u32,
    pub size: u32,
    pub o_address: u32,
    pub d_address: u32,
}