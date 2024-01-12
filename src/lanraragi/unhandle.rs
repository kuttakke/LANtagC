use super::archive::Archive;

use std::fs::{read, write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static NO_HANDLE_FILE_NAME: &str = "no_handle.json";

pub fn no_handle_file() -> &'static Mutex<Vec<Archive>> {
    static NO_HANDLE_FILE: OnceLock<Mutex<Vec<Archive>>> = OnceLock::new();
    NO_HANDLE_FILE.get_or_init(|| {
        // check if file exists
        if !Path::new(NO_HANDLE_FILE_NAME).exists() {
            return Mutex::new(Vec::new());
        }
        let no_handle_data = String::from_utf8(read(NO_HANDLE_FILE_NAME).unwrap()).unwrap();
        if no_handle_data.is_empty() {
            return Mutex::new(Vec::new());
        }
        Mutex::new(serde_json::from_str::<Vec<Archive>>(&no_handle_data).unwrap())
    })
}

pub fn add_and_save_no_handle(data: Archive) {
    let mut no_handle_array = no_handle_file().lock().unwrap();
    no_handle_array.push(data);
    write(
        NO_HANDLE_FILE_NAME,
        serde_json::to_string_pretty(&*no_handle_array)
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
}