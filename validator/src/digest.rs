use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{self, Read};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

pub const ZERO: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

#[cfg(target_os = "linux")]
const O_NOFOLLOW: i32 = 0o400000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW: i32 = 0x0100;
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
const O_NOFOLLOW: i32 = 0;

pub fn bytes(payload: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(payload))
}

pub fn file(path: &Path) -> Result<String, String> {
    read_file_bytes(path).map(|payload| bytes(&payload))
}

pub fn read_file_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let before = metadata_result(path, fs::symlink_metadata(path), "metadata failed")?;
    if before.file_type().is_symlink() || !before.file_type().is_file() {
        return Err(format!("{}: not a regular file", path.display()));
    }
    if is_hard_linked(&before) {
        return Err(format!("{}: hard-linked file rejected", path.display()));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(O_NOFOLLOW);
    let mut file = open_result(path, options.open(path))?;
    let opened = metadata_result(path, file.metadata(), "opened metadata failed")?;
    reject_changed_or_linked(path, &before, &opened)?;
    let mut payload = Vec::new();
    read_result(path, file.read_to_end(&mut payload))?;
    Ok(payload)
}

fn metadata_result(
    path: &Path,
    result: io::Result<fs::Metadata>,
    label: &str,
) -> Result<fs::Metadata, String> {
    result.map_err(|err| format!("{}: {label}: {err}", path.display()))
}

fn open_result(path: &Path, result: io::Result<fs::File>) -> Result<fs::File, String> {
    result.map_err(|err| format!("{}: open failed: {err}", path.display()))
}

fn read_result(path: &Path, result: io::Result<usize>) -> Result<(), String> {
    result
        .map(|_| ())
        .map_err(|err| format!("{}: read failed: {err}", path.display()))
}

pub fn canonical_json(value: &serde_json::Value) -> String {
    let sorted = sorted_value(value);
    self::bytes(&sorted.to_string().into_bytes())
}

fn sorted_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(sorted_value).collect()),
        Value::Object(items) => {
            let mut keys = items.keys().collect::<Vec<_>>();
            keys.sort();
            let mut sorted = Map::new();
            for key in keys {
                let item = items.get(key).expect("sorted key collected from object");
                sorted.insert(key.clone(), sorted_value(item));
            }
            Value::Object(sorted)
        }
        other => other.clone(),
    }
}

#[cfg(unix)]
fn metadata_changed(before: &fs::Metadata, opened: &fs::Metadata) -> bool {
    before.dev() != opened.dev() || before.ino() != opened.ino()
}

#[cfg(unix)]
fn is_hard_linked(meta: &fs::Metadata) -> bool {
    meta.nlink() > 1
}

#[cfg(not(unix))]
fn metadata_changed(before: &fs::Metadata, opened: &fs::Metadata) -> bool {
    before.len() != opened.len()
}

#[cfg(not(unix))]
fn is_hard_linked(_meta: &fs::Metadata) -> bool {
    false
}

fn reject_changed_or_linked(
    path: &Path,
    before: &fs::Metadata,
    opened: &fs::Metadata,
) -> Result<(), String> {
    if metadata_changed(before, opened) {
        return Err(format!("{}: file changed during open", path.display()));
    }
    if is_hard_linked(opened) {
        return Err(format!("{}: hard-linked file rejected", path.display()));
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::{
        metadata_result, open_result, read_file_bytes, read_result, reject_changed_or_linked,
    };
    use std::fs;
    use std::io::Error;
    use std::os::unix::fs::symlink;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn read_file_bytes_rejects_symlink_leaf() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ultragoal-digest-test-{stamp}"));
        fs::create_dir(&dir).expect("create temp dir");
        let real = dir.join("real.txt");
        let link = dir.join("link.txt");
        fs::write(&real, b"ok").expect("write real file");
        symlink(&real, &link).expect("create symlink");

        assert_eq!(read_file_bytes(&real).expect("read real"), b"ok");
        assert!(read_file_bytes(&link).is_err());

        fs::remove_dir_all(&dir).expect("remove temp dir");
    }

    #[test]
    fn read_file_bytes_rejects_hard_linked_leaf() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ultragoal-hardlink-test-{stamp}"));
        fs::create_dir(&dir).expect("create temp dir");
        let real = dir.join("real.txt");
        let link = dir.join("link.txt");
        fs::write(&real, b"ok").expect("write real file");
        fs::hard_link(&real, &link).expect("create hard link");

        assert!(read_file_bytes(&real).is_err());
        assert!(read_file_bytes(&link).is_err());

        fs::remove_dir_all(&dir).expect("remove temp dir");
    }

    #[test]
    fn opened_metadata_guard_rejects_changed_or_linked_files() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ultragoal-opened-meta-test-{stamp}"));
        fs::create_dir(&dir).expect("create temp dir");
        let before_path = dir.join("before.txt");
        let opened_path = dir.join("opened.txt");
        let hard_path = dir.join("hard.txt");
        fs::write(&before_path, b"before").expect("before");
        fs::write(&opened_path, b"opened").expect("opened");
        fs::write(&hard_path, b"hard").expect("hard");
        fs::hard_link(&hard_path, dir.join("hard-link.txt")).expect("hard link");
        let before = fs::metadata(&before_path).expect("before meta");
        let opened = fs::metadata(&opened_path).expect("opened meta");
        let hard = fs::metadata(&hard_path).expect("hard meta");

        assert!(
            reject_changed_or_linked(&before_path, &before, &opened)
                .expect_err("changed file rejected")
                .contains("file changed during open")
        );
        assert!(
            reject_changed_or_linked(&hard_path, &hard, &hard)
                .expect_err("linked file rejected")
                .contains("hard-linked file rejected")
        );
        assert!(reject_changed_or_linked(&before_path, &before, &before).is_ok());
        fs::remove_dir_all(&dir).expect("remove temp dir");
    }

    #[test]
    fn read_file_bytes_error_mappers_are_typed_and_testable() {
        let path = Path::new("proof.txt");
        assert!(
            metadata_result(path, Err(Error::other("missing")), "opened metadata failed")
                .expect_err("metadata error")
                .contains("opened metadata failed")
        );
        assert!(
            open_result(path, Err(Error::other("denied")))
                .expect_err("open error")
                .contains("open failed")
        );
        assert!(
            read_result(path, Err(Error::other("interrupted")))
                .expect_err("read error")
                .contains("read failed")
        );
    }
}
