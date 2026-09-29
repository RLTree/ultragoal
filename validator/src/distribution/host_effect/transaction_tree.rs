use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;

const TREE_LIMIT: usize = 64 * 1024 * 1024;

pub(crate) struct TreeObservation {
    pub(crate) present: bool,
    pub(crate) digest: String,
}

pub(crate) fn observe_tree(path: &Path) -> Result<TreeObservation, &'static str> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(TreeObservation {
                present: false,
                digest: digest(b"absent"),
            });
        }
        Err(_) => return Err("host observation root unavailable"),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("host observation root is not a directory");
    }
    let mut entries = Vec::new();
    let mut bytes = 0;
    walk(path, Path::new(""), &mut entries, &mut bytes)?;
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let mut canonical = Vec::new();
    for (relative, size, digest) in entries {
        canonical.extend_from_slice(relative.as_bytes());
        canonical.extend_from_slice(&size.to_le_bytes());
        canonical.extend_from_slice(digest.as_bytes());
        canonical.push(0);
    }
    let after =
        fs::symlink_metadata(path).map_err(|_| "host observation root revalidation failed")?;
    if after.file_type().is_symlink()
        || !after.is_dir()
        || metadata.dev() != after.dev()
        || metadata.ino() != after.ino()
        || metadata.nlink() != after.nlink()
    {
        return Err("host observation root identity changed");
    }
    Ok(TreeObservation {
        present: true,
        digest: digest(&canonical),
    })
}

pub(crate) fn observe_file(path: &Path) -> Result<TreeObservation, &'static str> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(TreeObservation {
                present: false,
                digest: digest(b"absent"),
            });
        }
        Err(_) => return Err("host observation file unavailable"),
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.file_type().is_fifo()
        || metadata.file_type().is_socket()
        || metadata.file_type().is_block_device()
        || metadata.file_type().is_char_device()
    {
        return Err("host observation file is not a regular unique file");
    }
    let bytes = read_stable(path, &metadata)?;
    Ok(TreeObservation {
        present: true,
        digest: digest(&bytes),
    })
}

fn walk(
    root: &Path,
    relative: &Path,
    entries: &mut Vec<(String, u64, String)>,
    total: &mut usize,
) -> Result<(), &'static str> {
    let path = root.join(relative);
    let mut children = fs::read_dir(&path)
        .map_err(|_| "host observation directory unavailable")?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "host observation entry unavailable")?;
    children.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
    for entry in children {
        let name = entry.file_name();
        let child_relative = relative.join(&name);
        let child = root.join(&child_relative);
        let metadata = fs::symlink_metadata(&child).map_err(|_| "host observation entry raced")?;
        let kind = metadata.file_type();
        if kind.is_symlink()
            || kind.is_fifo()
            || kind.is_socket()
            || kind.is_block_device()
            || kind.is_char_device()
        {
            return Err("host observation contains an unsafe entry");
        }
        if kind.is_dir() {
            walk(root, &child_relative, entries, total)?;
            continue;
        }
        if !kind.is_file() || metadata.nlink() != 1 {
            return Err("host observation contains a non-unique file");
        }
        let bytes = read_stable(&child, &metadata)?;
        *total = total
            .checked_add(bytes.len())
            .ok_or("host observation too large")?;
        if *total > TREE_LIMIT {
            return Err("host observation exceeds byte limit");
        }
        entries.push((
            child_relative.to_string_lossy().into_owned(),
            bytes.len() as u64,
            digest(&bytes),
        ));
    }
    Ok(())
}

fn read_stable(path: &Path, before: &fs::Metadata) -> Result<Vec<u8>, &'static str> {
    let mut file = File::open(path).map_err(|_| "host observation file open failed")?;
    let mut bytes = Vec::new();
    (&mut file)
        .take((TREE_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "host observation file read failed")?;
    if bytes.len() > TREE_LIMIT {
        return Err("host observation exceeds byte limit");
    }
    let after = file
        .metadata()
        .map_err(|_| "host observation file revalidation failed")?;
    if before.dev() != after.dev()
        || before.ino() != after.ino()
        || before.len() != after.len()
        || before.nlink() != after.nlink()
    {
        return Err("host observation file identity changed");
    }
    Ok(bytes)
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
