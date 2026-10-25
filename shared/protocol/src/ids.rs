use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "server", derive(sqlx::Type), sqlx(transparent))]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        #[cfg(feature = "server")]
        impl $name {
            /// Generates a new time-ordered id.
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
        }

        #[cfg(feature = "server")]
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Uuid::from_str(value).map(Self)
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }
    };
}

id_type!(
    /// Identifier of a user.
    UserId
);
id_type!(
    /// Identifier of a file or folder.
    NodeId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_parses_from_its_display_form() {
        let id = NodeId(Uuid::nil());
        assert_eq!(id.to_string().parse::<NodeId>(), Ok(id));
    }

    #[test]
    fn malformed_id_is_rejected() {
        assert!("not-an-id".parse::<NodeId>().is_err());
    }
}
