pub const HOTAGEKEY: u32 = 60 * 60 * 24;       // 1day
pub const COLDAGEKEY: u32 = 60 * 60 * 24 * 14; // 14day

pub enum GCStrategy {
    Forward,
    BackgroundSimple,
    BackgroundCold,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PageUsedStatus {
    Clean,
    Dirty,
    Busy(u32),
}