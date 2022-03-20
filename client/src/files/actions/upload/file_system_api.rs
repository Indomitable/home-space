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
    #[wasm_bindgen(catch)]
    pub async fn showDirectoryPicker() -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn showOpenFilePicker() -> Result<JsValue, JsValue>;
}

#[wasm_bindgen(module = "/js/file-upload.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    pub async fn uploadDataTransferItems(parent_id: i64, items: Array) -> Result<JsValue, JsValue>;
}