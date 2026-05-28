use std::sync::atomic::AtomicBool;

pub static IS_RUNNING: AtomicBool = AtomicBool::new(true);
