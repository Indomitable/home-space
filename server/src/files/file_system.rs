use std::path::{PathBuf, Path};
use std::fs::{self, File};
use std::io::{Result, Write};
use std::sync::Arc;
use actix_web::web::{self, Bytes};
use futures_util::{Stream, TryStreamExt};

use super::paths_manager::PathManager;

pub(crate) async fn execute_file_system_operation<TOutput>(operation: impl FnOnce() -> Result<TOutput> + Send + 'static) -> Result<TOutput>
where TOutput: Send + 'static {
    web::block(operation)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
}

pub(crate) struct FileSystemManager {
    user_id: i64,
    user_root_dir: PathBuf,
    trash_dir: PathBuf,
    versions_dir: PathBuf,
    path_manager: Arc<PathManager>,
}

impl FileSystemManager {
    pub(crate) fn new(user_id: i64, path_manager: &Arc<PathManager>) -> Self {
        Self {
            user_id,
            user_root_dir: path_manager.get_top_save_folder(user_id),
            trash_dir: path_manager.get_trash_dir(user_id),
            versions_dir: path_manager.get_version_dir(user_id),
            path_manager: Arc::clone(path_manager),
        }
    }

    /// Create folder
    pub(crate) async fn create_dir(&self, path: &str) -> Result<()> {
        let path = self.user_root_dir.join(path);
        execute_file_system_operation(move || fs::create_dir(&path)).await?;
        Ok(())
    }

    /// Remove folder
    pub(crate) async fn remove_dir(&self, path: &str) -> Result<()> {
        let path = self.user_root_dir.join(path);
        execute_file_system_operation(move || fs::remove_dir(&path)).await?;
        Ok(())
    }

    ///
    /// Copy node in another parent.
    /// Paths are relative to the user root.
    pub(crate) async fn copy_file_to_destination(&self, source_path: &str, destination_path: &str) -> Result<()> {
        let source = self.user_root_dir.join(source_path);
        let destination = self.user_root_dir.join(destination_path);

        execute_file_system_operation(move || fs::copy(&source, &destination)).await?;
        Ok(())
    }

    pub(crate) async fn move_file_to_destination(&self, source_path: &str, destination_path: &str) -> Result<()> {
        let source = self.user_root_dir.join(source_path);
        let destination = self.user_root_dir.join(destination_path);

        fn move_file(source_path: &Path, destination_path: &Path) -> Result<()> {
            fs::copy(source_path, destination_path)?;
            fs::remove_file(source_path)?;
            Ok(())
        }

        execute_file_system_operation(move || move_file(&source, &destination)).await?;
        Ok(())
    }

    /// Copy file to versions folder under version_file_name file name.
    /// Do nothing to folders, they don't have version.
    pub(crate) async fn copy_to_versions(&self, node_path: &str, version_file_name: &str) -> Result<()> {
        let source = self.user_root_dir.join(node_path);
        let destination = self.versions_dir.join(version_file_name);
        execute_file_system_operation(move || fs::copy(&source, &destination)).await?;
        Ok(())
    }

    pub(crate) async fn write_stream_to_file<TStream, TError>(&self, output: &PathBuf, mut stream: TStream) -> Result<usize>
        where TStream: Stream<Item=std::result::Result<Bytes, TError>> + Unpin {
        let mut size = 0_usize;
        {
            fn create_file(path: PathBuf) -> Result<File> {
                let f = File::create(path)?;
                Ok(f)
            }

            fn append_file(mut file: File, bytes: Bytes) -> Result<File> {
                let f = file.write_all(&bytes).map(|_| file)?;
                Ok(f)
            }

            let output = output.clone();
            let mut f = execute_file_system_operation(move || create_file(output)).await?;
            while let Ok(Some(chunk)) = stream.try_next().await {
                size += chunk.len();
                f = execute_file_system_operation(move || append_file(f, chunk)).await?;
            }
        }
        Ok(size)
    }
}

