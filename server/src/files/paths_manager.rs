use std::{path::PathBuf, fs};

use crate::config::get_files_location;

pub trait PathManager {
    fn get_top_save_folder(&self, user_id: i64) -> PathBuf;
    fn get_trash_dir(&self, user_id: i64) -> PathBuf;
    fn get_version_dir(&self, user_id: i64) -> PathBuf;
    fn init_user_fs(&self, user_id: i64) -> std::io::Result<()>;

    fn path_to_string(&self, path: &PathBuf) -> String;
}

pub struct PathManagerImpl {
}

pub fn path_manager_new() -> impl PathManager {
    PathManagerImpl {}
}

const SYSTEM_DIR: &'static str = ".system";
const TRASH_DIR: &'static str = "trash";
const VERSION_DIR: &'static str = "version";

impl PathManager for PathManagerImpl {
    fn get_top_save_folder(&self, user_id: i64) -> PathBuf {
        let files_location = get_files_location();
        let root: PathBuf = files_location.into();
        return root.join(user_id.to_string());
    }
    
    fn get_trash_dir(&self, user_id: i64) -> PathBuf {
        let save_dir: PathBuf = self.get_top_save_folder(user_id);
        return save_dir.join(SYSTEM_DIR).join(TRASH_DIR);
    }
    
    fn get_version_dir(&self, user_id: i64) -> PathBuf {
        let save_dir: PathBuf = self.get_top_save_folder(user_id);
        return save_dir.join(SYSTEM_DIR).join(TRASH_DIR);
    }

    fn init_user_fs(&self, user_id: i64) -> std::io::Result<()>  {
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

    fn path_to_string(&self, path: &PathBuf) -> String {
        path.as_os_str().to_str().unwrap().to_owned()
    }
}
