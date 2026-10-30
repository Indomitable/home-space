use iced::{
    Color, Element, Theme,
    widget::{svg, svg::Handle},
};

use protocol::{Node, NodeKind};

use crate::theme;

const IMAGE_PREFIX: &str = "image/";
const TEXT_PREFIX: &str = "text/";
const DOCUMENT_TYPES: &[&str] = &[
    "application/pdf",
    "application/rtf",
    "application/json",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.oasis.opendocument.text",
];

/// Default edge length of an icon in logical pixels.
pub const ICON_SIZE: f32 = 20.0;

/// The Material Symbols icons bundled with the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Folder,
    Image,
    Document,
    Other,
    Upload,
    Download,
    NewFolder,
    Rename,
    Move,
    Delete,
    Recycling,
    Restore,
    EmptyBin,
    Refresh,
    Up,
    Close,
    Info,
    Checked,
    Unchecked,
    Mixed,
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
}

/// How an icon is tinted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    Text,
    Muted,
    Accent,
    Success,
    Danger,
    Disabled,
}

impl Icon {
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Folder => include_bytes!("../assets/icons/folder.svg"),
            Self::Image => include_bytes!("../assets/icons/image.svg"),
            Self::Document => include_bytes!("../assets/icons/description.svg"),
            Self::Other => include_bytes!("../assets/icons/draft.svg"),
            Self::Upload => include_bytes!("../assets/icons/upload.svg"),
            Self::Download => include_bytes!("../assets/icons/download.svg"),
            Self::NewFolder => include_bytes!("../assets/icons/create_new_folder.svg"),
            Self::Rename => include_bytes!("../assets/icons/edit.svg"),
            Self::Move => include_bytes!("../assets/icons/drive_file_move.svg"),
            Self::Delete => include_bytes!("../assets/icons/delete.svg"),
            Self::Recycling => include_bytes!("../assets/icons/recycling.svg"),
            Self::Restore => include_bytes!("../assets/icons/restore_from_trash.svg"),
            Self::EmptyBin => include_bytes!("../assets/icons/delete_forever.svg"),
            Self::Refresh => include_bytes!("../assets/icons/refresh.svg"),
            Self::Up => include_bytes!("../assets/icons/arrow_upward.svg"),
            Self::Close => include_bytes!("../assets/icons/close.svg"),
            Self::Info => include_bytes!("../assets/icons/info.svg"),
            Self::Checked => include_bytes!("../assets/icons/check_box.svg"),
            Self::Unchecked => include_bytes!("../assets/icons/check_box_outline_blank.svg"),
            Self::Mixed => include_bytes!("../assets/icons/indeterminate_check_box.svg"),
            Self::Sync => include_bytes!("../assets/icons/sync.svg"),
            Self::FolderOpen => include_bytes!("../assets/icons/folder_open.svg"),
            Self::SyncPending => include_bytes!("../assets/icons/schedule.svg"),
            Self::Synced => include_bytes!("../assets/icons/check_circle.svg"),
            Self::SyncProblem => include_bytes!("../assets/icons/sync_problem.svg"),
            Self::SyncDisabled => include_bytes!("../assets/icons/sync_disabled.svg"),
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
    let palette = theme.extended_palette();
    match tint {
        Tint::Text => palette.background.base.text,
        Tint::Muted => palette.background.strongest.color,
        Tint::Accent => palette.primary.base.color,
        Tint::Success => palette.success.base.color,
        Tint::Danger => palette.danger.base.color,
        // The svg tint drops alpha, so blend with the background into an opaque colour.
        Tint::Disabled => mix(
            palette.background.base.color,
            palette.background.base.text,
            theme::DISABLED_ALPHA,
        ),
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

/// The icon that represents `node`, chosen by MIME type.
pub fn node_icon(node: &Node) -> Icon {
    match (node.kind, node.mime.as_str()) {
        (NodeKind::Folder, _) => Icon::Folder,
        (_, mime) if mime.starts_with(IMAGE_PREFIX) => Icon::Image,
        (_, mime) if mime.starts_with(TEXT_PREFIX) || DOCUMENT_TYPES.contains(&mime) => {
            Icon::Document
        }
        _ => Icon::Other,
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use protocol::NodeId;

    use super::*;

    fn node(kind: NodeKind, mime: Option<&str>) -> Node {
        Node {
            id: NodeId(Uuid::nil()),
            parent_id: None,
            kind,
            name: String::new(),
            size: 0,
            sha256: None,
            mime: mime.unwrap_or_default().to_owned(),
            version: 1,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn folders_use_the_folder_icon() {
        assert_eq!(node_icon(&node(NodeKind::Folder, None)), Icon::Folder);
    }

    #[test]
    fn images_use_the_image_icon() {
        assert_eq!(
            node_icon(&node(NodeKind::File, Some("image/png"))),
            Icon::Image
        );
    }

    #[test]
    fn text_and_documents_use_the_document_icon() {
        let icons: Vec<_> = ["text/plain", "application/pdf"]
            .into_iter()
            .map(|mime| node_icon(&node(NodeKind::File, Some(mime))))
            .collect();
        assert_eq!(icons, [Icon::Document, Icon::Document]);
    }

    #[test]
    fn everything_else_uses_the_other_icon() {
        let icons: Vec<_> = [Some("application/zip"), None]
            .into_iter()
            .map(|mime| node_icon(&node(NodeKind::File, mime)))
            .collect();
        assert_eq!(icons, [Icon::Other, Icon::Other]);
    }
}
