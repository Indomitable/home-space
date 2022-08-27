use actix_web::HttpRequest;
use actix_web::{web, Responder, Result, get, put};
use log::error;

use home_space_contracts::files::{CreateNode, CreateFolderRequest};
use crate::ioc::container::Contrainer;
use crate::response::*;
use crate::auth::AuthContext;
use crate::sorting::Sorting;

///
/// Method: GET 
/// `/api/files/nodes/{parent_id}`
/// parent_id = 0 => top node
/// parent_id > 0 -> sub nodes
/// 
#[get("/nodes/{parent_id}")]
pub(crate) async fn get_nodes(provider: web::Data<Contrainer>, path: web::Path<i64>, query: web::Query<Sorting>, user: AuthContext) -> Result<impl Responder> {
    let parent_id = path.into_inner();
    let sorting: Sorting = query.into_inner();
    let service = provider.get_node_provide_service(user.user_id);
    match service.list_nodes(parent_id, &sorting).await {
        Ok(nodes) =>Ok(web::Json(nodes)),
        Err(e) => {
            error!("Error has occurred while list nodes: {:?}", e);
            error_internal_server_error()
        }
    }
}

///
/// Downloads file with id.
/// 
#[get("/file/{id}")]
pub async fn get_file(provider: web::Data<Contrainer>, path: web::Path<i64>, user: AuthContext) -> Result<impl Responder> {
    let id = path.into_inner();
    let service = provider.get_node_provide_service(user.user_id);
    match service.get_file(id).await {
        Ok(file) => Ok(file),
        Err(e) => {
            error!("Error has occurred while getting file: {:?}", e);
            error_not_found() // file not found
        }
    }

}

///
/// Method: PUT
/// Creates folder, when parent_id is 0 then it is top folder.
#[put("/create-folder")]
pub async fn create_folder(provider: web::Data<Contrainer>, user: AuthContext, body: web::Json<CreateFolderRequest>) -> Result<impl Responder> {
    let CreateFolderRequest { parent_id, name } = body.into_inner();
    let service = provider.get_node_create_service(user.user_id);
    match service.create_folder_node(parent_id, &name).await {
        Ok(id) => {
            created(CreateNode { id })
        }
        Err(e) => {
            error!("Error has occurred while creating folder: {:?}", e);
            error_internal_server_error()
        }
    }
}

///
/// Method: DELETE
/// `/api/files/delete_node/{id}` delete node - if folder delete all contents
/// 
// #[delete("/delete-node/{id}")]
// pub async fn delete_node(provider: web::Data<Contrainer>, path: web::Path<i64>, user: AuthContext) -> Result<impl Responder> {
//     let id = path.into_inner();
//     let repo = provider.get_file_repository(user.user_id);
//     if let Ok(_) = repo.move_to_trash(id).await {
//         return no_content()
//         // // delete file from the file system only if it was deleted.
//
//         // if node.node_type == NODE_TYPE_FILE {
//         //     if execute_file_system_operation(move || trash_mover.move_file_to_trash(node.filesystem_path.into())).await.is_ok() {
//         //         return no_content()
//         //     }
//         // } else {
//         //     if execute_file_system_operation(move || trash_mover.move_dir_to_trash(node.filesystem_path.into())).await.is_ok() {
//         //         return no_content()
//         //     }
//         // }
//     }
//     error_not_found()
// }


// #[post("/move_node/{id}/{parent_id}")]
// pub async fn move_node(request: HttpRequest, provider: web::Data<Contrainer>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
//     todo!("Implement move node");
//     // let id = path.into_inner();
//     no_content()
// }

// #[post("/copy_node/{id}/{parent_id}")]
// pub async fn copy_node(request: HttpRequest, provider: web::Data<Contrainer>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
//     todo!("Implement copy node");
//     // let id = path.into_inner();
//     created()
// }

///
/// Method: PUT 
/// `/api/files/upload-file/0` upload file in top folder
/// `/api/files/upload-file/{parent_id}` parent_id > 0 upload file in sub folder
/// Creates a new file or if file exits it creates a new version of it.
///
#[put("/upload-file")]
pub async fn upload_file(provider: web::Data<Contrainer>, request: HttpRequest, user: AuthContext, body: web::Payload) -> Result<impl Responder> {
    let user_id = user.user_id;
    let file_name = read_string_header(&request, "X-FILE-NAME").expect("Request should have File name present");
    let parent_id = read_int_header(&request, "X-PARENT-ID").expect("Request should have parent id");
    let node_create = provider.get_node_create_service(user.user_id);
    match node_create.create_file_node(parent_id, file_name, body).await {
        Ok(id) => {
            created(CreateNode { id })
        }
        Err(e) => {
            error!("Error has occurred while uploading file: {:?}", e);
            error_internal_server_error()
        }
    }
}

// async fn write_request_to_file(output: &PathBuf, mut body: web::Payload) -> std::result::Result<i64, Box<dyn std::error::Error>> {
//     let mut size = 0_i64;
//     {
//         let output = output.clone();
//         let mut f = execute_file_system_operation(move || create_file(output)).await?;
//         while let Some(chunk) = body.try_next().await? {
//             size = size + chunk.len() as i64;
//             f = execute_file_system_operation(move || append_file(f, chunk)).await?;
//         }
//     }
//     Ok(size)
// }

#[get("/parents/{parent_id}")]
pub async fn get_parents(provider: web::Data<Contrainer>, path: web::Path<i64>, user: AuthContext )-> Result<impl Responder>  {
    let parent_id = path.into_inner();
    let service = provider.get_node_provide_service(user.user_id);
    match service.get_parent_nodes(parent_id).await {
        Ok(parents) => {
            json(parents)
        }
        Err(e) => {
            error!("Error has occurred while getting parents: {:?}", e);
            error_internal_server_error()
        }
    }
}

// #[derive(Debug, Deserialize)]
// pub struct TitleQueryString {
//     title: String,
// }
//
// #[get("/node-by-name/{parent_id}")]
// pub async fn get_node_by_name(provider: web::Data<Contrainer>, path: web::Path<i64>, query: web::Query<TitleQueryString>, user: AuthContext )-> Result<impl Responder>  {
//     let parent_id = path.into_inner();
//     let repo = provider.get_file_repository(user.user_id);
//     let search_query = SearchModel {
//         parent_id: Some(parent_id),
//         title: Some(query.title.clone()),
//         node_type: None,
//         mime_type: None,
//         from_date: None,
//         to_date: None,
//         from_size: None,
//         to_size: None
//     };
//     match repo.search_nodes(&search_query).await {
//         Ok(nodes) => json(nodes.iter().map(|n| 1).collect::<Vec::<i32>>()),
//         Err(e) => {
//             log::error!("Error finding node. [Error: {:?}]", e);
//             error_internal_server_error()
//         }
//     }
// }
//


