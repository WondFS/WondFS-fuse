use std::env;
use fuser::{Filesystem, MountOption};
mod tl;
mod gc;
mod kv;
mod buf;
mod fuse;
mod core;
mod driver;
mod util;
mod inode;
mod common;
mod compress;
mod write_buf;

#[macro_use]
extern crate log;

fn main() {
    env_logger::init();
    let mountpoint = env::args_os().nth(1).unwrap();
    let fs = fuse::fuse::WondFS::new();
    trace!("WondFS init success");
    fuser::mount2(fs, mountpoint, &[MountOption::AutoUnmount]).unwrap();
}