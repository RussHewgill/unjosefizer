#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_doc_comments)]
#![allow(unused_labels)]
// #![windows_subsystem = "windows"]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use unjosefizer_lib::{logging::init_logs, *};

fn main() {
    // test_main().unwrap();
    ui::run_eframe().unwrap();
}
