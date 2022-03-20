use std::{path::{PathBuf, Path}, fs, fs::{File, DirEntry}, error::Error, io::Write};
use uuid::Uuid;
use actix_web::web::{self, Bytes};

pub async fn create_file(path: PathBuf) -> Result<File, Box<dyn Error>> {
    let f = web::block(move || File::create(path)).await??;
    Ok(f)
}

pub async fn create_dir(path: PathBuf) -> Result<(), Box<dyn Error>> {
    Ok(web::block(move || fs::create_dir(path)).await??)
}

pub async fn append_file(mut file: File, bytes: Bytes) -> Result<File, Box<dyn Error>> {
    let f = web::block(move || file.write_all(&bytes).map(|_| file)).await??;
    Ok(f)
}

pub async fn delete_file(path: PathBuf) -> Result<(), Box<dyn Error>> {
    Ok(web::block(move || fs::remove_file(path)).await??)
}

pub async fn delete_dir_recurse(path: PathBuf) -> Result<(), Box<dyn Error>> {
    Ok(web::block(move || fs::remove_dir_all(path)).await??)
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

fn visit_files<TVisitor>(dir: PathBuf, file_visitor: TVisitor) -> Result<(), Box<dyn Error>>
where TVisitor: Fn(&DirEntry) -> Result<(), Box<dyn Error>> {
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
pub async fn move_dir_to_trash(dir: PathBuf, root: PathBuf) -> Result<(), Box<dyn Error>> {
    let trash_dir = Path::join(&root, ".trash");
    if !trash_dir.exists() {
        fs::create_dir(trash_dir)?;
    }

    let _ = visit_files(dir, |file| {
        let filename = Uuid::new_v4().to_hyphenated().to_string();
        let _ = std::fs::copy(file.path(), &filename);
        let _ = std::fs::remove_file(file.path());
        Ok(())
    });

    Ok(())
}