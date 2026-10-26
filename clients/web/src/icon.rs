use leptos::prelude::*;

use protocol::{Node, NodeKind};

const ICON_FOLDER: &str = "folder";
const ICON_IMAGE: &str = "image";
const ICON_DOCUMENT: &str = "description";
const ICON_OTHER: &str = "draft";

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

/// The Material Symbols ligature that represents `node`, chosen by MIME type.
pub fn node_icon(node: &Node) -> &'static str {
    match (node.kind, node.mime.as_str()) {
        (NodeKind::Folder, _) => ICON_FOLDER,
        (_, mime) if mime.starts_with(IMAGE_PREFIX) => ICON_IMAGE,
        (_, mime) if mime.starts_with(TEXT_PREFIX) || DOCUMENT_TYPES.contains(&mime) => {
            ICON_DOCUMENT
        }
        _ => ICON_OTHER,
    }
}

#[component]
pub fn Icon(name: &'static str) -> impl IntoView {
    view! {
        <span class="icon" class:icon--folder=name == ICON_FOLDER aria-hidden="true">
            {name}
        </span>
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use protocol::NodeId;

    use super::*;

    fn node(kind: NodeKind, mime: Option<&str>) -> Node {
        Node {
            id: NodeId::from(uuid::Uuid::nil()),
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
        assert_eq!(node_icon(&node(NodeKind::Folder, None)), "folder");
    }

    #[test]
    fn images_use_the_image_icon() {
        assert_eq!(node_icon(&node(NodeKind::File, Some("image/png"))), "image");
    }

    #[test]
    fn text_and_documents_use_the_description_icon() {
        let icons: Vec<_> = ["text/plain", "application/pdf"]
            .into_iter()
            .map(|mime| node_icon(&node(NodeKind::File, Some(mime))))
            .collect();
        assert_eq!(icons, ["description", "description"]);
    }

    #[test]
    fn everything_else_uses_the_draft_icon() {
        let icons: Vec<_> = [
            Some("application/zip"),
            Some("application/octet-stream"),
            None,
        ]
        .into_iter()
        .map(|mime| node_icon(&node(NodeKind::File, mime)))
        .collect();
        assert_eq!(icons, ["draft", "draft", "draft"]);
    }
}
