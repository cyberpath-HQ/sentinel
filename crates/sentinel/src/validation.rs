/// Windows reserved names that cannot be used as filenames.
/// These names are reserved by the Windows operating system and will cause
/// filesystem errors if used as directory or file names.
pub const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
    "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Checks if a name contains only valid filesystem-safe characters.
///
/// Valid characters are: alphanumeric, underscore (_), hyphen (-), and dot (.).
/// Invalid characters are: path separators (/ \), control characters, and Windows reserved
/// characters (< > : " | ? *).
pub fn is_valid_name_chars(name: &str) -> bool {
    for ch in name.chars() {
        match ch {
            // Path separators
            '/' | '\\' => return false,
            // Control characters
            '\0' ..= '\x1F' | '\x7F' => return false,
            // Windows reserved characters
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => return false,
            // Valid characters: alphanumeric, underscore, hyphen, dot
            'a' ..= 'z' | 'A' ..= 'Z' | '0' ..= '9' | '_' | '-' | '.' => {},
            // Any other character is invalid
            _ => return false,
        }
    }
    true
}

/// Checks if a string contains valid characters for a document ID.
/// Document IDs disallow dots to avoid confusion with file extensions.
pub fn is_valid_document_id_chars(name: &str) -> bool {
    for ch in name.chars() {
        match ch {
            // Path separators
            '/' | '\\' => return false,
            // Control characters
            '\0' ..= '\x1F' | '\x7F' => return false,
            // Windows reserved characters
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => return false,
            // Valid characters: alphanumeric, underscore, hyphen
            'a' ..= 'z' | 'A' ..= 'Z' | '0' ..= '9' | '_' | '-' => {},
            // Any other character is invalid
            _ => return false,
        }
    }
    true
}

/// Checks if a name is a Windows reserved name (case-insensitive).
///
/// Reserved names include CON, PRN, AUX, NUL, COM1-9, LPT1-9.
/// Also checks the base name before any extension.
pub fn is_reserved_name(name: &str) -> bool {
    let name_upper = name.to_uppercase();
    // Check both the full name and the base name (before first dot)
    let base_name = name_upper.split('.').next().unwrap_or(&name_upper);
    WINDOWS_RESERVED_NAMES.contains(&name_upper.as_str()) || WINDOWS_RESERVED_NAMES.contains(&base_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_name_chars() {
        assert!(is_valid_name_chars("valid_name-123.txt"));
        assert!(!is_valid_name_chars("invalid/name"));
        assert!(!is_valid_name_chars("invalid\\name"));
        assert!(!is_valid_name_chars("invalid<name>"));
        assert!(!is_valid_name_chars("invalid:name"));
        assert!(!is_valid_name_chars("invalid|name"));
        assert!(!is_valid_name_chars("invalid?name"));
        assert!(!is_valid_name_chars("invalid*name"));
        assert!(!is_valid_name_chars("invalid\u{0001}name")); // Control character
        assert!(!is_valid_name_chars("invalid\u{FFFF}name")); // Control character
    }
}
