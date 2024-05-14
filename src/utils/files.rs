use std::{fs::File, io::Read};

pub fn file_exists(file_name: &str) -> bool {
    File::open(file_name).is_ok()
}


pub fn read_file(file_name: &str) -> std::io::Result<String> {
    let mut file = File::open(file_name)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}