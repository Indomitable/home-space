use std::{path::PathBuf, fs};

use crate::config::get_files_location;

pub(crate) struct PathManager {}

const SYSTEM_DIR: &str = ".system";
const TRASH_DIR: &str = "trash";
const VERSION_DIR: &str = "version";

impl PathManager {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn get_top_save_folder(&self, user_id: i64) -> PathBuf {
        let files_location = get_files_location();
        let root: PathBuf = files_location.into();
        return root.join(user_id.to_string());
    }

    pub(crate) fn get_trash_dir(&self, user_id: i64) -> PathBuf {
        let save_dir: PathBuf = self.get_top_save_folder(user_id);
        return save_dir.join(SYSTEM_DIR).join(TRASH_DIR);
    }

    pub(crate) fn get_version_dir(&self, user_id: i64) -> PathBuf {
        let save_dir: PathBuf = self.get_top_save_folder(user_id);
        return save_dir.join(SYSTEM_DIR).join(VERSION_DIR);
    }

    pub(crate) fn path_to_string(&self, path: &PathBuf) -> String {
        path.into_os_string().into_string().expect("Path should be in UTF-8 format")
    }

    pub(crate) fn init_user_fs(&self, user_id: i64) -> std::io::Result<()>  {
        let user_files_root: PathBuf = self.get_top_save_folder(user_id);
        let system_dir = user_files_root.join(SYSTEM_DIR);
        let trash_dir = system_dir.join(TRASH_DIR);
        let versions_dir = system_dir.join(VERSION_DIR);

        fs::create_dir(user_files_root)?;
        fs::create_dir(system_dir)?;
        fs::create_dir(trash_dir)?;
        fs::create_dir(versions_dir)?;
        Ok(())
    }
}
