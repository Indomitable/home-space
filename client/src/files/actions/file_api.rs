use gloo_utils::window;
use wasm_bindgen::prelude::*;
use js_sys::{Reflect, AsyncIterator, IteratorNext};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

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
    #[wasm_bindgen(catch)]
    async fn showDirectoryPicker() -> Result<JsValue, JsValue>;

    #[wasm_bindgen()]
    pub fn showOpenFilePicker();
}

#[wasm_bindgen(module = "/js/file-upload.js")]
extern "C" {
    #[wasm_bindgen(js_name=FileUpload, skip_typescript)]
    #[derive(Debug, Clone, PartialEq)]
    pub type FileUpload;

    #[wasm_bindgen()]
    pub fn defineFileUpload();

    // #[wasm_bindgen(constructor)]
    // fn new() -> Tester;

    // #[wasm_bindgen (method, js_class = "Tester" , js_name=test)]
    // pub fn test(this: &Tester);
}


pub async fn get_directory() {
    // let tester = Tester::new();
    // tester.test();

    // if let Ok(true) = is_file_api_supported() {
    //     if let Ok(directory_handler) = showDirectoryPicker().await {
    //         let handle: FileSystemDirectoryHandle = directory_handler.dyn_into().expect("Cast");
    //         debug!("{}", handle.kind());

    //         let async_iterator = handle.values();
    //         while let Ok(promise) = async_iterator.next() {
    //             match JsFuture::from(promise).await {
    //                 Ok(val) => {
    //                     let dir: IteratorNext =  val.unchecked_into::<IteratorNext>(); // Use unchecked_into because it is plain js object and instanceof will not return true.
    //                     if dir.done() {
    //                         return;
    //                     } else {
    //                         let handle: FileSystemHandle = dir.value().dyn_into().expect("Should be file system handle");
    //                         debug!("{}", handle.name());
    //                     }
    //                 },
    //                 Err(e) => {
    //                     error!("{:?}", e);
    //                     break;
    //                 }
    //             }
    //         }
    //     }
    // }
}