use gloo_utils::window;
use wasm_bindgen::prelude::*;
use js_sys::{Reflect, AsyncIterator, Array};

pub fn is_file_api_supported() -> bool {
    if let Ok(true) = Reflect::has(&window(), &JsValue::from_str("showDirectoryPicker")) {
        true
    } else {
        false
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name=FileSystemHandle, skip_typescript)]
    #[derive(Debug, Clone, PartialEq)]
    pub type FileSystemHandle;
    #[wasm_bindgen (method, getter, js_class = "FileSystemHandle" , js_name=name)]
    pub fn name(this: &FileSystemHandle) -> String;
    #[wasm_bindgen (method, getter, js_class = "FileSystemHandle" , js_name=kind)]
    pub fn kind(this: &FileSystemHandle) -> String;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends=FileSystemHandle, js_name=FileSystemDirectoryHandle, skip_typescript)]
    #[derive(Debug, Clone, PartialEq)]
    pub type FileSystemDirectoryHandle;
    #[wasm_bindgen (method, js_class = "FileSystemDirectoryHandle" , js_name=keys)]
    pub fn keys(this: &FileSystemDirectoryHandle) -> AsyncIterator;

    #[wasm_bindgen (method, js_class = "FileSystemDirectoryHandle" , js_name=values)]
    pub fn values(this: &FileSystemDirectoryHandle) -> AsyncIterator;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends=FileSystemHandle, js_name=FileSystemFileHandle, skip_typescript)]
    #[derive(Debug, Clone, PartialEq)]
    pub type FileSystemFileHandle;

    #[wasm_bindgen(method, js_class = "FileSystemFileHandle", js_name=getFile, catch)]
    pub async fn getFile(this: &FileSystemFileHandle) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_name="showDirectoryPicker")]
    pub async fn show_directory_picker() -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_name="showOpenFilePicker")]
    pub async fn show_open_file_picker(options: JsValue) -> Result<JsValue, JsValue>;
}


#[wasm_bindgen(module = "/js/file-upload.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name="uploadDataTransferItems")]
    pub async fn upload_data_transfer_items(parent_id: JsValue, items: Array) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_name="uploadDirectoryHandle")]
    pub async fn upload_directory_handle(parent_id: JsValue, directory_handle: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_name="uploadFile")]
    pub async fn upload_file(parent_id: JsValue, file: JsValue) -> Result<JsValue, JsValue>;
}

// Implement it in js, where it whould be simplier to handle the async iterator and the recursive async and there we have upload file and create folder methods.
// pub async fn select_directory_and_upload() {
//     if let Ok(directory_handle) = show_directory_picker().await {
//         upload_directory(directory_handle);
//     }
// }

// async fn upload_directory(directory_handle: JsValue) {
//     let handle: FileSystemDirectoryHandle = directory_handle.unchecked_into();

//     let async_iterator = handle.values();
//     while let Ok(promise) = async_iterator.next() {
//         match JsFuture::from(promise).await {
//             Ok(val) => {
//                 let dir: IteratorNext =  val.unchecked_into::<IteratorNext>(); // Use unchecked_into because it is plain js object and instanceof will not return true.
//                 if dir.done() {
//                     return;
//                 } else {
//                     let handle: FileSystemHandle = dir.value().dyn_into().expect("Should be file system handle");
//                     if handle.kind() == "directory" {
//                         upload_directory(handle.obj).await;
//                     } else {
                        
//                     }
//                 }
//             },
//             Err(e) => {
//                 error!("{:?}", e);
//                 break;
//             }
//         }
//     }
// }