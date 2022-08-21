use std::path::PathBuf;
use std::fs::{self, File};
use std::io::{Result, Write};
use actix_web::web::{self, Bytes};

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

pub fn move_file(source_path: &PathBuf, destination_path: &PathBuf) -> Result<()> {
    fs::copy(source_path, destination_path)?;
    fs::remove_file(source_path)?;
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

