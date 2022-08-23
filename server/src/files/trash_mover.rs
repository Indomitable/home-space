use std::path::PathBuf;
use std::io::Result;

use async_trait::async_trait;
use home_space_contracts::files::NODE_TYPE_FILE;

use super::db::deleted_node::DeletedNodeDto;
use super::file_system::{execute_file_system_operation, move_file, delete_dir};
use super::paths_manager::PathManager;

#[async_trait]
pub(crate) trait TrashMover {
    async fn move_node_to_trash(&self, node: &DeletedNodeDto) -> Result<()>;
    async fn restore_node_from_trash(&self, node: &DeletedNodeDto) -> Result<()>;
    // fn move_file_to_trash(&self, path: PathBuf) -> Result<()>;
    // fn move_dir_to_trash(&self, dir: PathBuf) -> Result<()>;
}

pub(crate) struct TrashMoverImpl {
    trash_dir: PathBuf,
    user_root_dir: PathBuf,
}

pub(crate) fn trash_mover_new<TPathManager: PathManager>(user_id: i64, path_manager: TPathManager) -> impl TrashMover {
    TrashMoverImpl {
        trash_dir: path_manager.get_trash_dir(user_id),
        user_root_dir: path_manager.get_top_save_folder(user_id)
    }
}

impl TrashMoverImpl {
//     fn visit(&self, path: PathBuf) -> Result<()> {
//         let filename = Uuid::new_v4().as_hyphenated().to_string();
//         fs::copy(&path, self.trash_dir.join(filename))?;
//         fs::remove_file(path)?;
//         Ok(())
//     }

//     fn visit_files(&self, dir: PathBuf) -> Result<()> {
//         if dir.is_dir() {
//             for entry in fs::read_dir(dir)? {
//                 let entry = entry?;
//                 let path = entry.path();
//                 self.visit_files(path)?;
//             }
//         } else {
//             self.visit(dir)?;
//         }
//         Ok(())
//     }


}

#[async_trait]
impl TrashMover for TrashMoverImpl {
    async fn move_node_to_trash(&self, node: &DeletedNodeDto) -> Result<()> {
        let source = self.user_root_dir.join(&node.filesystem_path);
        if node.node_type == NODE_TYPE_FILE {
            let destination = self.trash_dir.join(&node.file_name);
            execute_file_system_operation(move || move_file(&source, &destination)).await?;
        } else {
            let source = self.user_root_dir.join(&node.filesystem_path);
            execute_file_system_operation(move || delete_dir(&source)).await?;
        }
        Ok(())
    }

    async fn restore_node_from_trash(&self, node: &DeletedNodeDto) -> Result<()> {
        todo!();
    }

    // fn move_dir_to_trash(&self, dir: PathBuf) -> Result<()> {
    //     self.visit_files(dir)?;
    //     Ok(())
    // }
}
