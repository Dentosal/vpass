/// Human readable validation error
#[derive(Debug, Clone)]
pub enum ValidationError {
    Empty,
    InvalidCharacters,
    InvalidPattern,
}

/// Validate vault name, [a-zA-Z0-9._]+ no adjacent dots
#[must_use]
pub fn vault_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        Err(ValidationError::Empty)
    } else if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "_.".contains(c))
    {
        Err(ValidationError::InvalidCharacters)
    } else if name.contains("..") {
        Err(ValidationError::InvalidPattern)
    } else {
        Ok(())
    }
}
