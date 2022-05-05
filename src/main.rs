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

fn main() {
    env_logger::init();
}