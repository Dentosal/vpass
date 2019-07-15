use crate::cli::error::{Error, VResult};

/// Human readable validation error
#[derive(Debug, Clone)]
pub enum ValidationError {
    Empty,
    InvalidCharacters,
    InvalidPattern,
}

/// Validate vault name, [a-zA-Z0-9._]+ no adjacent dots
#[must_use]
pub fn vault_name(name: &str) -> VResult<()> {
    if name.is_empty() {
        Err(Error::VaultNameInvalid(ValidationError::Empty))
    } else if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "_.".contains(c))
    {
        Err(Error::VaultNameInvalid(ValidationError::InvalidCharacters))
    } else if name.contains("..") {
        Err(Error::VaultNameInvalid(ValidationError::InvalidPattern))
    } else {
        Ok(())
    }
}

/// Validate item name, [a-zA-Z0-9/_]+ no adjacent, leading or trailing slashes
#[must_use]
pub fn item_name(name: &str) -> VResult<()> {
    if name.is_empty() {
        Err(Error::VaultNameInvalid(ValidationError::Empty))
    } else if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "_.".contains(c))
    {
        Err(Error::VaultNameInvalid(ValidationError::InvalidCharacters))
    } else if name.contains("..") {
        Err(Error::VaultNameInvalid(ValidationError::InvalidPattern))
    } else {
        Ok(())
    }
}
