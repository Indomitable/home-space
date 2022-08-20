use std::{path::PathBuf, fs};
use std::io::Result;

use uuid::Uuid;

use super::paths_manager::PathManager;

pub trait TrashMover {
    fn move_file_to_trash(&self, path: PathBuf) -> Result<()>;
    fn move_dir_to_trash(&self, dir: PathBuf) -> Result<()>;
}

pub struct TrashMoverImpl {
    trash_dir: PathBuf,
}

pub fn trash_mover_new<TPathManager: PathManager>(user_id: i64, path_manager: TPathManager) -> impl TrashMover {
    TrashMoverImpl {
        trash_dir: path_manager.get_trash_dir(user_id),
    }
}

impl TrashMoverImpl {
    fn visit(&self, path: PathBuf) -> Result<()> {
        let filename = Uuid::new_v4().as_hyphenated().to_string();
        fs::copy(&path, self.trash_dir.join(filename))?;
        fs::remove_file(path)?;
        Ok(())
    }

    fn visit_files(&self, dir: PathBuf) -> Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                self.visit_files(path)?;
            }
        } else {
            self.visit(dir)?;
        }
        Ok(())
    }
}

impl TrashMover for TrashMoverImpl {
    fn move_file_to_trash(&self, path: PathBuf) -> Result<()> {
        let filename = Uuid::new_v4().as_hyphenated().to_string();
        fs::copy(&path, self.trash_dir.join(filename))?;
        fs::remove_file(path)?;
        Ok(())
    }

    fn move_dir_to_trash(&self, dir: PathBuf) -> Result<()> {
        self.visit_files(dir)?;
        Ok(())
    }
}
