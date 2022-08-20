use std::{path::PathBuf, fs};

use uuid::Uuid;

use super::paths_manager::PathManager;

pub trait VersionsMover {
    fn move_to_versions(&self, source_path: &PathBuf) -> std::io::Result<String>;
}

pub struct VersionsMoverImpl {
    versions_dir: PathBuf,
}

pub fn versions_mover_new<PM>(user_id: i64, path_manager: PM) -> impl VersionsMover
where PM: PathManager {
    VersionsMoverImpl {
        versions_dir: path_manager.get_version_dir(user_id),
    }
}


impl VersionsMover for VersionsMoverImpl {
    fn move_to_versions(&self, source_path: &PathBuf) -> std::io::Result<String> {
        let destination_name = Uuid::new_v4().as_simple().to_string();
        let destination_path = self.versions_dir.join(&destination_name);
        fs::copy(&source_path, destination_path)?;
        fs::remove_file(source_path)?;
        return Ok(destination_name);
    }
}
