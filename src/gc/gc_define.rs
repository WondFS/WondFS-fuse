pub const HOTAGEKEY: u32 = 60 * 60 * 24;       // 1day
pub const COLDAGEKEY: u32 = 60 * 60 * 24 * 14; // 14day

// GC Strategy Type
pub enum GCStrategy {
    Forward,          // 前台GC
    BackgroundSimple, // 后台标准GC
    BackgroundCold,   // 后台回收冷块
}

// Page Used Status Type
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PageUsedStatus {
    Clean,
    Dirty,
    Busy(u32),
}