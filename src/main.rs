use std::env;
use fuser::{Filesystem, MountOption};
mod gc;
mod kv;
mod buf;
mod raw;
mod fuse;
mod core;
mod driver;
mod util;
mod inode;
mod common;
mod sys_file;
mod fake_proc;
mod super_stat;

#[macro_use]
extern crate log;

struct NullFS;

impl Filesystem for NullFS {}

fn main() {
    env_logger::init();
    let mountpoint = env::args_os().nth(1).unwrap();
    let fs = fuse::fuse::WondFS::new();
    trace!("WondFS init success");
    fuser::mount2(fs, mountpoint, &[MountOption::AutoUnmount]).unwrap();
}