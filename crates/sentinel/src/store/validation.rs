use tracing::{trace, warn};

use crate::{
    validation::{is_reserved_name, is_valid_name_chars},
    CollectionWalConfigOverrides,
    Result,
    SentinelError,
};

/// - Must not be empty
/// - Must not contain path separators (`/` or `\`)
/// - Must not contain control characters (0x00-0x1F, 0x7F)
/// - Must not be a Windows reserved name (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
/// - Must not start with a dot (.) to avoid hidden directories
/// - Must only contain alphanumeric characters, underscores (_), hyphens (-), and dots (.)
/// - Must not end with a dot or space (Windows limitation)
///
/// # Parameters
/// - `name`: The collection name to validate
///
/// # Returns
/// - `Ok(())` if the name is valid
/// - `Err(SentinelError::InvalidCollectionName)` if the name is invalid
///
/// # Examples
/// ```no_run
/// # use sentinel_dbms::{Store, SentinelError};
/// # use std::path::Path;
/// # async fn example() -> Result<(), SentinelError> {
/// let store = Store::new(Path::new("/tmp/test"), None).await?;
///
/// // Valid names
/// assert!(store
///     .collection_with_config(
///         "users",
///         Some(CollectionWalConfigOverrides::default())
///     )
///     .await
///     .is_ok());
/// assert!(store
///     .collection_with_config(
///         "user_data",
///         Some(CollectionWalConfigOverrides::default())
///     )
///     .await
///     .is_ok());
/// assert!(store
///     .collection_with_config(
///         "data-2024",
///         Some(CollectionWalConfigOverrides::default())
///     )
///     .await
///     .is_ok());
/// assert!(store
///     .collection_with_config(
///         "test_collection_123",
///         Some(CollectionWalConfigOverrides::default())
///     )
///     .await
///     .is_ok());
///
/// // Invalid names
/// assert!(store
///     .collection_with_config(
///         "",
///         Some(CollectionWalConfigOverrides::default())
///     )
///     .await
///     .is_err());
/// assert!(store
///     .collection_with_config(
///         ".hidden",
///         Some(CollectionWalConfigOverrides::default())
///     )
///     .await
///     .is_err());
/// assert!(store
///     .collection_with_config(
///         "path/traversal",
///         Some(CollectionWalConfigOverrides::default())
///     )
///     .await
///     .is_err());
/// assert!(store
///     .collection_with_config(
///         "CON",
///         Some(CollectionWalConfigOverrides::default())
///     )
///     .await
///     .is_err());
/// # Ok(())
/// # }
/// ```
pub fn validate_collection_name(name: &str) -> Result<()> {
    use tracing::debug;
    debug!("Validating collection name: {}", name);
    // Check if name is empty
    if name.is_empty() {
        debug!("Collection name is empty");
        return Err(SentinelError::InvalidCollectionName {
            name: name.to_owned(),
        });
    }

    // Check if name starts with a dot (hidden directory)
    if name.starts_with('.') && name != ".keys" {
        debug!("Collection name starts with dot and is not .keys: {}", name);
        return Err(SentinelError::InvalidCollectionName {
            name: name.to_owned(),
        });
    }

    // Check if name ends with a dot or space (Windows limitation)
    if name.ends_with('.') || name.ends_with(' ') {
        warn!("Collection name ends with dot or space: {}", name);
        return Err(SentinelError::InvalidCollectionName {
            name: name.to_owned(),
        });
    }

    // Check for valid characters
    if !is_valid_name_chars(name) {
        debug!("Collection name contains invalid characters: {}", name);
        return Err(SentinelError::InvalidCollectionName {
            name: name.to_owned(),
        });
    }

    // Check for Windows reserved names
    if is_reserved_name(name) {
        debug!("Collection name is a reserved name: {}", name);
        return Err(SentinelError::InvalidCollectionName {
            name: name.to_owned(),
        });
    }

    trace!("Collection name '{}' is valid", name);
    Ok(())
}
