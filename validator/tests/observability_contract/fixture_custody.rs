use std::fs;
use std::path::{Path, PathBuf};

pub struct TestDir {
    path: PathBuf,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl TestDir {
    pub fn new(label: &str) -> Self {
        let root =
            fs::canonicalize(std::env::temp_dir()).expect("resolve operating-system temp root");
        let label = safe_label(label);
        let path = (0..32)
            .find_map(|_| {
                let mut random = [0_u8; 16];
                getrandom::fill(&mut random).expect("obtain fixture custody randomness");
                let candidate = root.join(format!("hul-observability-{label}-{}", hex(&random)));
                match create_private_directory(&candidate) {
                    Ok(()) => Some(candidate),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => None,
                    Err(error) => panic!("create private observability fixture: {error}"),
                }
            })
            .expect("allocate unique observability fixture custody");
        let metadata = fs::symlink_metadata(&path).expect("capture observability fixture identity");
        assert!(
            metadata.is_dir() && !metadata.file_type().is_symlink(),
            "fixture custody must be a real directory"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            Self {
                path,
                device: metadata.dev(),
                inode: metadata.ino(),
            }
        }
        #[cfg(not(unix))]
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn store_path(&self) -> PathBuf {
        self.path.join("events.jsonl")
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let Ok(metadata) = fs::symlink_metadata(&self.path) else {
            return;
        };
        if metadata.is_dir()
            && !metadata.file_type().is_symlink()
            && fixture_identity_matches(self, &metadata)
        {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[cfg(unix)]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    fs::create_dir(path)
}

#[cfg(unix)]
fn fixture_identity_matches(dir: &TestDir, metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    metadata.dev() == dir.device && metadata.ino() == dir.inode
}

#[cfg(not(unix))]
fn fixture_identity_matches(_dir: &TestDir, _metadata: &fs::Metadata) -> bool {
    true
}

fn safe_label(label: &str) -> String {
    label
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                char::from(DIGITS[usize::from(*byte >> 4)]),
                char::from(DIGITS[usize::from(*byte & 0x0f)]),
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_custody_is_private_random_and_checked_on_cleanup() {
        let first = TestDir::new("custody");
        let second = TestDir::new("custody");
        assert_ne!(first.path(), second.path());
        let temp_root = fs::canonicalize(std::env::temp_dir()).unwrap();
        assert_eq!(first.path().parent(), Some(temp_root.as_path()));
        assert_eq!(second.path().parent(), Some(temp_root.as_path()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = fs::symlink_metadata(first.path())
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0);
        }
        let first_path = first.path().to_owned();
        let second_path = second.path().to_owned();
        drop(first);
        drop(second);
        assert!(!first_path.exists());
        assert!(!second_path.exists());
    }

    #[test]
    fn cleanup_refuses_replacement_directory_identity() {
        let dir = TestDir::new("replacement");
        let path = dir.path().to_owned();
        let held = path.with_extension("held");
        fs::rename(&path, &held).unwrap();
        create_private_directory(&path).unwrap();
        fs::write(path.join("outside-sentinel"), b"preserve").unwrap();

        drop(dir);

        assert_eq!(
            fs::read(path.join("outside-sentinel")).unwrap(),
            b"preserve"
        );
        fs::remove_dir_all(path).unwrap();
        fs::remove_dir_all(held).unwrap();
    }
}
