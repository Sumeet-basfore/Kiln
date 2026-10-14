//! Raw file reads and permission-preserving atomic writes.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_ID: AtomicU64 = AtomicU64::new(1);

/// Reads the exact bytes stored at a path.
pub fn read_file(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    fs::read(path)
}

/// Writes bytes with a same-directory temporary file and atomic rename.
pub fn save_file(path: impl AsRef<Path>, bytes: &[u8]) -> io::Result<()> {
    save_file_with_hook(path.as_ref(), bytes, || Ok(()))
}

fn save_file_with_hook<F>(path: &Path, bytes: &[u8], before_rename: F) -> io::Result<()>
where
    F: FnOnce() -> io::Result<()>,
{
    let target = resolve_save_target(path)?;
    let parent = target.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "file path has no parent directory",
        )
    })?;
    let name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "file path has no name"))?;
    let permissions = match fs::metadata(&target) {
        Ok(metadata) => Some(metadata.permissions()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let (temp_path, mut temp_file) = create_temp_file(parent, name)?;

    let result = (|| {
        temp_file.write_all(bytes)?;
        temp_file.flush()?;
        if let Some(permissions) = permissions {
            temp_file.set_permissions(permissions)?;
        }
        drop(temp_file);
        before_rename()?;
        fs::rename(&temp_path, &target)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}

fn resolve_save_target(path: &Path) -> io::Result<PathBuf> {
    let metadata = fs::symlink_metadata(path);
    match metadata {
        Ok(metadata) if metadata.file_type().is_symlink() => fs::canonicalize(path),
        Ok(_) => Ok(path.to_path_buf()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(path.to_path_buf()),
        Err(error) => Err(error),
    }
}

fn create_temp_file(parent: &Path, target_name: &std::ffi::OsStr) -> io::Result<(PathBuf, File)> {
    for _ in 0..100 {
        let id = TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let mut temp_name = target_name.to_os_string();
        temp_name.push(format!(".kiln-tmp-{}-{id}", std::process::id()));
        let temp_path = parent.join(temp_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique temporary save file",
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos());
            let path =
                std::env::temp_dir().join(format!("kiln-file-io-{}-{stamp}", std::process::id()));
            fs::create_dir_all(&path).expect("test directory creation succeeds");
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn read_returns_exact_file_bytes() {
        let dir = TestDir::new();
        let path = dir.0.join("text.txt");
        fs::write(&path, b"one\r\ntwo\r\n").expect("fixture write succeeds");
        assert_eq!(read_file(path).expect("read succeeds"), b"one\r\ntwo\r\n");
    }

    #[test]
    fn save_preserves_permissions() {
        let dir = TestDir::new();
        let path = dir.0.join("save.txt");
        fs::write(&path, b"before\n").expect("fixture write succeeds");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o640))
                .expect("permission setup succeeds");
        }

        save_file(&path, b"after\n").expect("atomic save succeeds");
        assert_eq!(read_file(&path).expect("read succeeds"), b"after\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(path)
                    .expect("metadata succeeds")
                    .permissions()
                    .mode()
                    & 0o777,
                0o640
            );
        }
    }

    #[test]
    fn interrupted_save_child_process() {
        if let Some(target) = std::env::var_os("KILN_TEST_INTERRUPTED_SAVE_TARGET") {
            let path = PathBuf::from(target);
            let _ = save_file_with_hook(&path, b"replacement", || std::process::abort());
        }
    }

    #[test]
    fn save_during_process_termination_keeps_original() {
        let dir = TestDir::new();
        let path = dir.0.join("target.txt");
        fs::write(&path, b"original").expect("fixture write succeeds");
        let status =
            std::process::Command::new(std::env::current_exe().expect("test executable path"))
                .args(["--exact", "file_io::tests::interrupted_save_child_process"])
                .env("KILN_TEST_INTERRUPTED_SAVE_TARGET", &path)
                .status()
                .expect("child test process starts");

        assert!(!status.success());
        assert_eq!(
            read_file(&path).expect("original file remains"),
            b"original"
        );
    }

    #[test]
    fn missing_parent_failure_does_not_create_target() {
        let dir = TestDir::new();
        let path = dir.0.join("missing-parent").join("file.txt");
        assert!(save_file(&path, b"new").is_err());
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn explicit_symlink_path_preserves_link_and_writes_target() {
        use std::os::unix::fs::symlink;
        let dir = TestDir::new();
        let target = dir.0.join("target.txt");
        let link = dir.0.join("link.txt");
        fs::write(&target, b"old").expect("fixture write succeeds");
        symlink(&target, &link).expect("symlink setup succeeds");

        save_file(&link, b"new").expect("explicit symlink save succeeds");
        assert_eq!(read_file(target).expect("read succeeds"), b"new");
        assert!(fs::symlink_metadata(link)
            .expect("metadata succeeds")
            .file_type()
            .is_symlink());
    }
}
