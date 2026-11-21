//! Newtypes for every identifier that crosses a crate boundary.
//!
//! A `PhotoId` is a stable 64-bit number assigned by the catalog, never a row offset and never a
//! path. Paths move; ids do not.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident, $inner:ty) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub $inner);

        impl $name {
            /// The numeric value, for storage and logs.
            pub const fn value(self) -> $inner {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }
    };
}

id_type! {
    /// A photo, video or sidecar-bearing frame in the catalog.
    PhotoId, u64
}
id_type! {
    /// A watched folder or subfolder.
    FolderId, u32
}
id_type! {
    /// A keyword.
    TagId, u32
}
id_type! {
    /// A collection the user assembled by hand.
    CollectionId, u32
}
id_type! {
    /// A mounted volume, identified by its filesystem id rather than its mount point.
    VolumeId, u32
}
id_type! {
    /// A background job, used to cancel and to report progress.
    JobId, u64
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_transparent_in_json() {
        let id = PhotoId(42);
        let text = serde_json::to_string(&id).expect("serialisable");
        assert_eq!(text, "42");
        let back: PhotoId = serde_json::from_str(&text).expect("deserialisable");
        assert_eq!(back, id);
    }

    #[test]
    fn display_is_the_number() {
        assert_eq!(FolderId(7).to_string(), "7");
    }
}
