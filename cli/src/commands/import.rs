use std::path::PathBuf;

pub fn import_command(source: &PathBuf, user: &str, destination: &str) {
    println!("Import: {} into {}:{}", source.as_os_str().to_str().unwrap(), user, destination);
}
