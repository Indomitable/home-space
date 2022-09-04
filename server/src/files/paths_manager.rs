use std::{path::PathBuf, fs};
use std::path::Path;
use uuid::Uuid;

use crate::config::get_files_location;
use crate::files::db::file_node::FileNodeDto;

pub(crate) struct PathManager {}

const SYSTEM_DIR: &str = ".system";
const TRASH_DIR: &str = "trash";
const VERSION_DIR: &str = "version";
const TEMP_DIR: &str = "temp";

impl PathManager {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn get_top_save_folder(&self, user_id: i64) -> PathBuf {
        let files_location = get_files_location();
        let root: PathBuf = files_location.into();
        root.join(user_id.to_string())
    }

    pub(crate) fn get_trash_dir(&self, user_id: i64) -> PathBuf {
        let save_dir: PathBuf = self.get_top_save_folder(user_id);
        save_dir.join(SYSTEM_DIR).join(TRASH_DIR)
    }

    pub(crate) fn get_version_dir(&self, user_id: i64) -> PathBuf {
        let save_dir: PathBuf = self.get_top_save_folder(user_id);
        save_dir.join(SYSTEM_DIR).join(VERSION_DIR)
    }

    pub(crate) fn get_temp_dir(&self, user_id: i64) -> PathBuf {
        let save_dir: PathBuf = self.get_top_save_folder(user_id);
        save_dir.join(SYSTEM_DIR).join(TEMP_DIR)
    }

    pub(crate) fn get_temp_file(&self, user_id: i64) -> PathBuf {
        let file_name = Uuid::new_v4().as_hyphenated().to_string();
        self.get_temp_dir(user_id).join(file_name)
    }

    pub(crate) fn path_to_string<P: AsRef<Path>>(&self, path: P) -> String {
        path.as_ref().to_str().expect("Path should be in UTF-8 format").to_owned()
    }

    pub(crate) fn get_absolute_path(&self, node: &FileNodeDto) -> PathBuf {
        self.get_top_save_folder(node.user_id).join(&node.filesystem_path)
    }

    pub(crate) fn rename(&self, node: &FileNodeDto, name: &str) -> String {
        let new_path = PathBuf::from(&node.filesystem_path).with_file_name(name);
        self.path_to_string(new_path)
    }

    pub(crate) fn init_user_fs(&self, user_id: i64) -> std::io::Result<()>  {
        let user_files_root: PathBuf = self.get_top_save_folder(user_id);
        let system_dir = user_files_root.join(SYSTEM_DIR);
        let trash_dir = system_dir.join(TRASH_DIR);
        let versions_dir = system_dir.join(VERSION_DIR);
        let temp_dir = system_dir.join(TEMP_DIR);

        fs::create_dir(user_files_root)?;
        fs::create_dir(system_dir)?;
        fs::create_dir(trash_dir)?;
        fs::create_dir(versions_dir)?;
        fs::create_dir(temp_dir)?;
        Ok(())
    }
}
