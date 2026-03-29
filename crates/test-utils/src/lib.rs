use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// A temporary database directory that cleans up on drop.
/// Every test that touches disk should use this to get an isolated directory.
pub struct TestDir {
    _dir: TempDir,
    path: PathBuf,
}

impl TestDir {
    /// Create a new temporary directory for a test.
    pub fn new(test_name: &str) -> Self {
        let dir = tempfile::Builder::new()
            .prefix(&format!("puredb-test-{test_name}-"))
            .tempdir()
            .expect("failed to create temp dir");
        let path = dir.path().to_path_buf();
        Self { _dir: dir, path }
    }

    /// Path to the temporary directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Convenience: return a file path within the temp directory.
    pub fn file(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}
