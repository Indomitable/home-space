use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Platform of a desktop client on Linux.
pub const PLATFORM_LINUX: &str = "desktop-linux";
/// Platform of a desktop client on macOS.
pub const PLATFORM_MACOS: &str = "desktop-macos";
/// Platform of a desktop client on Windows.
pub const PLATFORM_WINDOWS: &str = "desktop-windows";
/// Platform of the web client.
pub const PLATFORM_WEB: &str = "web";
/// Platform of the Android app.
pub const PLATFORM_ANDROID: &str = "android";

/// The device a client signs in from. The client generates `id` once and keeps it, so the
/// server recognises the device across sign-ins.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: Uuid,
    /// The name shown in the device list until the user renames it, such as the host name.
    pub name: String,
    /// One of the `PLATFORM_*` values, or another short name.
    pub platform: String,
}

/// A device of the user, from `GET /devices`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub id: Uuid,
    pub name: String,
    pub platform: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub last_seen_at: OffsetDateTime,
    /// Whether the device has a session that has not ended.
    pub signed_in: bool,
    /// Whether this is the device that asked.
    pub current: bool,
    /// How many resumable uploads it has left unfinished.
    pub uploads: i64,
}

/// Body of `PATCH /devices/{id}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameDevice {
    pub name: String,
}

#[cfg(test)]
mod tests {
    use crate::LoginRequest;

    #[test]
    fn a_login_without_a_device_is_still_accepted() {
        let request: LoginRequest =
            serde_json::from_str(r#"{"username":"alice","password":"pw"}"#).expect("json");
        assert_eq!(request.device, None);
    }
}
