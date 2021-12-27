/// A mockable interface to the filesystem
pub trait FilesystemTrait {
    /// Like std::path::Path::is_file()
    fn is_file(&self, path: &str) -> bool;
    /// Like std::fs::read_to_string()
    fn read_to_string(&self, path: &str) -> std::io::Result<String>;
}

pub type Filesystem = Box<dyn FilesystemTrait>;

pub fn real_filesystem() -> Filesystem {
    Box::new(FilesystemImpl)
}

struct FilesystemImpl;

impl FilesystemTrait for FilesystemImpl {
    fn is_file(&self, path: &str) -> bool {
        std::path::Path::new(path).is_file()
    }

    fn read_to_string(&self, path: &str) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_path(parts: &[&str]) -> String {
        let mut buf = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for part in parts {
            buf.push(part);
        }
        buf.to_str().unwrap().to_string()
    }

    #[test]
    fn test_file_is_file() {
        let path = workspace_path(&["Cargo.toml"]);
        let io = real_filesystem();
        assert!(io.is_file(&path));
    }

    #[test]
    fn test_not_file_is_not_file() {
        let path = workspace_path(&["not-a-file.txt"]);
        let io = real_filesystem();
        assert!(!io.is_file(&path));
    }

    #[test]
    fn test_directory_is_not_file() {
        let path = workspace_path(&["src"]);
        let io = real_filesystem();
        assert!(!io.is_file(&path));
    }

    #[test]
    fn test_can_read_file_to_string() {
        let path = workspace_path(&["Cargo.toml"]);
        let io = real_filesystem();
        let contents = io.read_to_string(&path).unwrap();
        assert!(contents.starts_with("[package]\n"));
    }

    #[test]
    fn test_can_not_read_not_file_to_string() {
        let path = workspace_path(&["not-a-file.txt"]);
        let io = real_filesystem();
        let err = io.read_to_string(&path).unwrap_err();
        assert!(err.kind() == std::io::ErrorKind::NotFound);
    }
}
