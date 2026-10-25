use std::io::{self, Read};

/// Why a provider could not produce text for a file.
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("reading content failed: {0}")]
    Io(#[from] io::Error),
    #[error("unsupported or corrupt content: {0}")]
    Unsupported(String),
}

/// Extracts searchable text from one family of file types.
///
/// Providers are registered with [`crate::SearchIndex`]; the first one that accepts a file
/// indexes its content. Bump [`version`](Self::version) when extraction changes so existing
/// files are re-extracted.
pub trait ContentProvider: Send + Sync + 'static {
    /// Stable identifier stored with the index entry.
    fn id(&self) -> &'static str;

    /// Version of the extraction logic.
    fn version(&self) -> u32;

    /// Whether this provider can read a file with this MIME type and name.
    fn accepts(&self, mime: &str, name: &str) -> bool;

    /// Reads the file and returns its text. Runs on a blocking thread and should stop reading
    /// once `limit` bytes of text have been produced.
    fn extract(&self, reader: &mut dyn Read, limit: usize) -> Result<String, ExtractError>;
}

pub(crate) fn tag(provider: &dyn ContentProvider) -> String {
    format!("{}@{}", provider.id(), provider.version())
}
