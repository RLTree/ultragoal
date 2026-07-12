use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use sha2::{Digest, Sha256};
use std::path::{Component, Path};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as platform;

#[cfg(not(unix))]
mod fallback;
#[cfg(not(unix))]
use fallback as platform;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ObjectIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) length: u64,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanos: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanos: i64,
    pub(crate) sha256: String,
}

#[derive(Clone, Debug)]
struct RecordedRead {
    path: String,
    maximum: usize,
    identity: ObjectIdentity,
}

pub(crate) struct ReadSession {
    root: platform::Root,
    records: Vec<RecordedRead>,
}

impl ReadSession {
    pub(crate) fn open(root: &Path) -> Result<Self, DistributionError> {
        Ok(Self {
            root: platform::open_root(root)?,
            records: Vec::new(),
        })
    }

    pub(crate) fn read(
        &mut self,
        path: &str,
        maximum: usize,
    ) -> Result<Vec<u8>, DistributionError> {
        validate_relative_path(path)?;
        let (bytes, identity) = platform::read(&self.root, path, maximum)?;
        self.records.push(RecordedRead {
            path: path.to_owned(),
            maximum,
            identity,
        });
        Ok(bytes)
    }

    pub(crate) fn finish(self) -> Result<(), DistributionError> {
        for record in self.records {
            let (_, current) = platform::read(&self.root, &record.path, record.maximum)?;
            if current != record.identity {
                return Err(error(DistributionErrorId::ObjectChanged));
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_relative_path(path: &str) -> Result<(), DistributionError> {
    if path.is_empty()
        || path.len() > 512
        || path.contains('\\')
        || !path.is_ascii()
        || path.bytes().any(|byte| {
            !(byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
        })
    {
        return Err(error(DistributionErrorId::InvalidPath));
    }
    let mut count = 0usize;
    for component in Path::new(path).components() {
        let Component::Normal(value) = component else {
            return Err(error(DistributionErrorId::InvalidPath));
        };
        if value == "." || value == ".." {
            return Err(error(DistributionErrorId::InvalidPath));
        }
        count += 1;
    }
    if count == 0 || count > 64 || path.split('/').any(str::is_empty) {
        return Err(error(DistributionErrorId::InvalidPath));
    }
    Ok(())
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn changed_object_between_read_and_session_seal_is_rejected() {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-distribution-reader-race-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("object.json"), b"first").unwrap();
        let mut session = ReadSession::open(&root).unwrap();
        assert_eq!(session.read("object.json", 64).unwrap(), b"first");
        fs::write(root.join("object.json"), b"second").unwrap();
        assert_eq!(
            session.finish().unwrap_err().id(),
            DistributionErrorId::ObjectChanged
        );
        fs::remove_dir_all(root).unwrap();
    }
}
