use js_sys::{Array, Date, Function, Intl::DateTimeFormat, Object, Reflect};
use time::OffsetDateTime;
use wasm_bindgen::JsValue;

/// Formats `time` in the browser's regional settings, in the local time zone.
pub fn short_date(time: OffsetDateTime) -> String {
    let millis = (time.unix_timestamp_nanos() / 1_000_000) as f64;
    let date = Date::new(&JsValue::from_f64(millis));
    let options = Object::new();
    for (key, value) in [("dateStyle", "short"), ("timeStyle", "short")] {
        let _ = Reflect::set(&options, &key.into(), &value.into());
    }
    let format: Function = DateTimeFormat::new(&Array::new(), &options).format();
    format
        .call1(&JsValue::NULL, &date)
        .ok()
        .and_then(|text| text.as_string())
        .unwrap_or_else(|| api_client::format::short_date(time))
}
