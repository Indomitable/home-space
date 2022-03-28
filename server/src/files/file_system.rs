use std::path::PathBuf;
use std::fs::{self, {File, DirEntry}};
use std::io::{Result, Write};
use uuid::Uuid;
use actix_web::web::{self, Bytes};

use crate::config::get_top_save_folder;

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

pub fn delete_file(path: PathBuf) -> Result<()> {
    fs::remove_file(path)
}

pub fn delete_dir_recurse(path: PathBuf) -> Result<()> {
    fs::remove_dir_all(path)
}

const SYSTEM_DIR: &'static str = ".system";
const TRASH_DIR: &'static str = "trash";
const VERSION_DIR: &'static str = "version";

pub fn get_trash_dir(root: &PathBuf) -> PathBuf {
    return root.join(SYSTEM_DIR).join(TRASH_DIR);
}

pub fn get_version_dir(root: &PathBuf) -> PathBuf {
    return root.join(SYSTEM_DIR).join(VERSION_DIR);
}

pub fn init_user_fs(user_files_root: PathBuf) -> Result<()>  {
    let system_dir = user_files_root.join(SYSTEM_DIR);
    let trash_dir = get_trash_dir(&user_files_root);
    let versions_dir = get_version_dir(&user_files_root);

    fs::create_dir(user_files_root)?;
    fs::create_dir(system_dir)?;
    fs::create_dir(trash_dir)?;
    fs::create_dir(versions_dir)?;
    Ok(())
}

pub fn move_file(source_path: &PathBuf, destination_path: &PathBuf) -> Result<()> {
    let source_path = source_path.clone();
    let destination_path = destination_path.clone();
    fs::copy(&source_path, destination_path)?;
    fs::remove_file(source_path)?;
    Ok(())
}

pub fn move_to_versions(source_path: &PathBuf, user_id: i64) -> Result<String> {
    let destination_name = Uuid::new_v4().to_simple().to_string();
    let root = get_top_save_folder(user_id).into();
    let destination_path = get_version_dir(&root).join(&destination_name);
    move_file(&source_path, &destination_path);
    return Ok(destination_name);
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

fn visit_files<TVisitor>(dir: PathBuf, file_visitor: TVisitor) -> Result<()>
where TVisitor: Fn(&DirEntry) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
              //  dir_visitor(&entry)?;
                visit_files(path, &file_visitor)?;
            } else {
                file_visitor(&entry)?;
            }
        }
    }
    Ok(())
}

/// Moving to files the trash, will move files with names changed to guids, this guids should be set in trash table.
pub fn move_dir_to_trash(dir: PathBuf, root: PathBuf) -> Result<()> {
    let trash_dir = get_trash_dir(&root);
    if !trash_dir.exists() {
        fs::create_dir(trash_dir)?;
    }

    visit_files(dir, |file| {
        let filename = Uuid::new_v4().to_hyphenated().to_string();
        fs::copy(file.path(), &filename)?;
        fs::remove_file(file.path())?;
        Ok(())
    })
}

pub async fn execute_file_system_operation<TOutput>(operation: impl FnOnce() -> Result<TOutput> + Send + 'static) -> std::result::Result<TOutput, Box<dyn std::error::Error>>
where TOutput: Send + 'static {
    let res = web::block(operation).await??;
    Ok(res)
}
