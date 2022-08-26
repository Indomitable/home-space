use async_trait::async_trait;
use std::path::{PathBuf, Path};
use std::fs::{self, File};
use std::io::{Result, Write};
use actix_web::web::{self, Bytes};

use super::paths_manager::PathManager;

pub fn create_file(path: PathBuf) -> Result<File> {
    let f = File::create(path)?;
    Ok(f)
}

pub fn create_dir(path: PathBuf) -> Result<()> {
    fs::create_dir(path)
}

pub fn append_file(mut file: File, bytes: Bytes) -> Result<File> {
    let f = file.write_all(&bytes).map(|_| file)?;
    Ok(f)
}

// pub fn delete_file(path: PathBuf) -> Result<()> {
//     fs::remove_file(path)
// }

pub fn delete_dir(path: &PathBuf) -> Result<()> {
    fs::remove_dir(path)
}

/// Moves file or empty folder to new location
pub fn move_node(source_path: &Path, destination_path: &Path) -> Result<()> {
    if source_path.is_file() {
        fs::copy(source_path, destination_path)?;
        fs::remove_file(source_path)?;
    } else {
        fs::create_dir(destination_path);
        fs::remove_dir(source_path);
    }
    Ok(())
}

/// Copies file or empty folder to new location
pub fn copy_node(source_path: &Path, destination_path: &Path) -> Result<()> {
    if source_path.is_file() {
        fs::copy(source_path, destination_path)?;
    } else {
        fs::create_dir(destination_path);
    }
    Ok(())
}

// fn visit_dirs_async<TVisitor, TVisitorOutput>(dir: PathBuf, dir_visitor: TVisitor, file_visitor: TVisitor) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error>>>>>
// where TVisitor: Fn(&DirEntry) -> TVisitorOutput + 'static,
//       TVisitorOutput: Future<Output = Result<(), Box<dyn Error>>> {
//     Box::pin(async {
//         if dir.is_dir() {
//             for entry in fs::read_dir(dir)? {
//                 let entry = entry?;
//                 let path = entry.path();
//                 if path.is_dir() {
//                     dir_visitor(&entry).await?;
//                     visit_dirs_async(path, &dir_visitor, &file_visitor).await?;
//                 } else {
//                     file_visitor(&entry).await?;
//                 }
//             }
//         }
//         Ok(())
//     })
// }

pub async fn execute_file_system_operation<TOutput>(operation: impl FnOnce() -> Result<TOutput> + Send + 'static) -> std::io::Result<TOutput>
where TOutput: Send + 'static {
    web::block(operation)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
}

#[async_trait]
pub(crate) trait FileSystemManager {
    async fn create_dir(&self, path: &str) -> Result<()>;

    async fn copy_node_to_destination(&self, source_path: &str, destination_path: &str) -> Result<()>;
    async fn copy_to_versions(&self, node_path: &str, version_file_name: &str) -> Result<()>;
}

pub(crate) struct FileSystemManagerImpl {
    user_root_dir: PathBuf,
    trash_dir: PathBuf,
    versions_dir: PathBuf,
}

impl FileSystemManagerImpl {
    pub(crate) fn new<TPathManager: PathManager>(user_id: i64, path_manager: TPathManager) -> Self {
        Self {
            user_root_dir: path_manager.get_top_save_folder(user_id),
            trash_dir: path_manager.get_trash_dir(user_id),
            versions_dir: path_manager.get_version_dir(user_id),
        }
    }
}

#[async_trait]
impl FileSystemManager for FileSystemManagerImpl {
    /// Create folder
    async fn create_dir(&self, path: &str) -> Result<()> {
        let path = self.user_root_dir.join(path);
        execute_file_system_operation(move || fs::create_dir(&path)).await?;
        Ok(())
    }

    ///
    /// Copy node in another parent.
    /// Paths are relative to the user root.
    async fn copy_node_to_destination(&self, source_path: &str, destination_path: &str) -> Result<()> {
        let source = self.user_root_dir.join(source_path);
        let destination = self.user_root_dir.join(destination_path);
        execute_file_system_operation(move || copy_node(&source, &destination)).await?;
        Ok(())
    }

    /// Copy file to versions folder under version_file_name file name.
    /// Do nothing to folders, they don't have version.
    async fn copy_to_versions(&self, node_path: &str, version_file_name: &str) -> Result<()> {
        let source = self.user_root_dir.join(node_path);
        let destination = self.versions_dir.join(version_file_name);
        execute_file_system_operation(move || fs::copy(&source, &destination)).await?;
        Ok(())
    }
}
