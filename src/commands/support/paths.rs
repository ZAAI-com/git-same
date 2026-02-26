use std::path::{Path, PathBuf};

/// Expands ~ in a path.
pub(crate) fn expand_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    let expanded = shellexpand::tilde(&path_str);
    PathBuf::from(expanded.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_path_absolute() {
        let path = Path::new("/tmp/some/path");
        assert_eq!(expand_path(path), PathBuf::from("/tmp/some/path"));
    }

    #[test]
    fn test_expand_path_tilde() {
        let path = Path::new("~/foo");
        let expanded = expand_path(path);
        assert!(!expanded.to_string_lossy().contains('~'));
        assert!(expanded.to_string_lossy().ends_with("/foo"));
    }
}
