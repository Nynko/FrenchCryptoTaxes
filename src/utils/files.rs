use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

pub fn file_exists(file_name: &str) -> bool {
    File::open(file_name).is_ok()
}

pub fn read_file(file_name: &str) -> std::io::Result<String> {
    let mut file = File::open(file_name)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

/* Create directories if they don't exist */
pub fn create_directories_if_needed(file_path: &str) {
    if let Some(parent) = Path::new(&file_path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).expect("Failed to create directories");
        }
    }
}
