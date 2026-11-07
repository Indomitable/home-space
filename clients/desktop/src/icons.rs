use client_core::icon::NodeIcon;
use iced::{
    Color, Element, Theme,
    widget::{svg, svg::Handle},
};

use protocol::Node;

use crate::theme;

/// Default edge length of an icon in logical pixels.
pub const ICON_SIZE: f32 = 20.0;

/// The Material Symbols icons bundled with the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Folder,
    FolderOutline,
    /// The app's mark.
    Home,
    Search,
    MoreVert,
    Image,
    Document,
    Other,
    Upload,
    Download,
    NewFolder,
    Rename,
    Move,
    Delete,
    Restore,
    EmptyBin,
    Refresh,
    Up,
    Close,
    Info,
    Checked,
    Unchecked,
    Mixed,
    More,
    Sync,
    FolderOpen,
    /// Sync badge: waiting to sync.
    SyncPending,
    /// Sync badge: in sync.
    Synced,
    /// Sync badge: syncing failed.
    SyncProblem,
    /// Sync badge: set to not sync.
    SyncDisabled,
    /// Opens something in the web browser.
    Web,
    /// Waiting for the sync.
    Update,
    SignOut,
    /// The user's devices.
    Devices,
}

/// How an icon is tinted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    Text,
    Muted,
    Accent,
    OnAccent,
    Success,
    Danger,
    Disabled,
}

impl Icon {
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Folder => include_bytes!("../assets/icons/folder_fill.svg"),
            Self::FolderOutline => include_bytes!("../assets/icons/folder.svg"),
            Self::Home => include_bytes!("../assets/icons/home.svg"),
            Self::Search => include_bytes!("../assets/icons/search.svg"),
            Self::MoreVert => include_bytes!("../assets/icons/more_vert.svg"),
            Self::Image => include_bytes!("../assets/icons/image.svg"),
            Self::Document => include_bytes!("../assets/icons/description.svg"),
            Self::Other => include_bytes!("../assets/icons/draft.svg"),
            Self::Upload => include_bytes!("../assets/icons/upload.svg"),
            Self::Download => include_bytes!("../assets/icons/download.svg"),
            Self::NewFolder => include_bytes!("../assets/icons/create_new_folder.svg"),
            Self::Rename => include_bytes!("../assets/icons/edit.svg"),
            Self::Move => include_bytes!("../assets/icons/drive_file_move.svg"),
            Self::Delete => include_bytes!("../assets/icons/delete.svg"),
            Self::Restore => include_bytes!("../assets/icons/restore_from_trash.svg"),
            Self::EmptyBin => include_bytes!("../assets/icons/delete_forever.svg"),
            Self::Refresh => include_bytes!("../assets/icons/refresh.svg"),
            Self::Up => include_bytes!("../assets/icons/arrow_upward.svg"),
            Self::Close => include_bytes!("../assets/icons/close.svg"),
            Self::Info => include_bytes!("../assets/icons/info.svg"),
            Self::Checked => include_bytes!("../assets/icons/check_box.svg"),
            Self::Unchecked => include_bytes!("../assets/icons/check_box_outline_blank.svg"),
            Self::Mixed => include_bytes!("../assets/icons/indeterminate_check_box.svg"),
            Self::More => include_bytes!("../assets/icons/more_horiz.svg"),
            Self::Sync => include_bytes!("../assets/icons/sync.svg"),
            Self::FolderOpen => include_bytes!("../assets/icons/folder_open.svg"),
            Self::SyncPending => include_bytes!("../assets/icons/schedule.svg"),
            Self::Synced => include_bytes!("../assets/icons/check_circle.svg"),
            Self::SyncProblem => include_bytes!("../assets/icons/sync_problem.svg"),
            Self::SyncDisabled => include_bytes!("../assets/icons/sync_disabled.svg"),
            Self::Web => include_bytes!("../assets/icons/open_in_new.svg"),
            Self::Update => include_bytes!("../assets/icons/update.svg"),
            Self::SignOut => include_bytes!("../assets/icons/logout.svg"),
            Self::Devices => include_bytes!("../assets/icons/devices.svg"),
        }
    }

    /// The icon at `size`, tinted from the current theme.
    pub fn view<'a, Message: 'a>(self, size: f32, tint: Tint) -> Element<'a, Message> {
        svg(Handle::from_memory(self.bytes()))
            .width(size)
            .height(size)
            .style(move |theme: &Theme, _status| svg::Style {
                color: Some(tint_color(theme, tint)),
            })
            .into()
    }
}

fn tint_color(theme: &Theme, tint: Tint) -> Color {
    let t = theme::tokens(theme);
    match tint {
        Tint::Text => t.text,
        Tint::Muted => t.muted,
        Tint::Accent => t.accent,
        Tint::OnAccent => t.on_accent,
        Tint::Success => t.success,
        Tint::Danger => t.danger,
        // The svg tint drops alpha, so blend with the background into an opaque colour.
        Tint::Disabled => mix(t.surface, t.text, theme::DISABLED_ALPHA),
    }
}

/// `from` moved `amount` of the way towards `to`.
fn mix(from: Color, to: Color, amount: f32) -> Color {
    let channel = |a: f32, b: f32| a + (b - a) * amount;
    Color::from_rgb(
        channel(from.r, to.r),
        channel(from.g, to.g),
        channel(from.b, to.b),
    )
}

/// The icon that represents `node`.
pub fn node_icon(node: &Node) -> Icon {
    match client_core::icon::node_icon(node) {
        NodeIcon::Folder => Icon::Folder,
        NodeIcon::Image => Icon::Image,
        NodeIcon::Document => Icon::Document,
        NodeIcon::Other => Icon::Other,
    }
}
