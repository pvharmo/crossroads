use std::fs;

pub mod interfaces;
pub mod providers;
pub mod storage;

pub fn read_token() -> String {
    fs::read_to_string("./token".to_string())
        .expect("Should have been able to read the file")
}