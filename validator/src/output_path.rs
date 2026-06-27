use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path};

#[cfg(target_os = "linux")]
const O_NOFOLLOW: i32 = 0o400000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW: i32 = 0x0100;
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
const O_NOFOLLOW: i32 = 0;

pub fn prepare_parent(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("output path is empty".to_string());
    }
    if path
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(format!(
            "{}: output path uses parent segment",
            path.display()
        ));
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        reject_existing_symlink_components(parent)?;
        fs::create_dir_all(parent)
            .map_err(|err| format!("{}: create parent failed: {err}", parent.display()))?;
        reject_symlink_components(parent)?;
    }
    reject_symlink_components(path)?;
    Ok(())
}

pub fn create_file(path: &Path, label: &str) -> Result<File, String> {
    prepare_parent(path)?;
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.custom_flags(O_NOFOLLOW);
    let file = options
        .open(path)
        .map_err(|err| format!("{}: {label} create failed: {err}", path.display()))?;
    reject_symlink_components(path)?;
    Ok(file)
}

pub fn write(path: &Path, bytes: impl AsRef<[u8]>, label: &str) -> Result<(), String> {
    let (tmp, mut file) = create_temp_file(path, label)?;
    write_result(path, label, file.write_all(bytes.as_ref()))?;
    sync_result(&tmp, label, file.sync_all())?;
    drop(file);
    finish_temp_file(&tmp, path, label)
}

pub(crate) fn write_result(path: &Path, label: &str, result: io::Result<()>) -> Result<(), String> {
    result.map_err(|err| format!("{}: {label} write failed: {err}", path.display()))
}

pub(crate) fn sync_result(path: &Path, label: &str, result: io::Result<()>) -> Result<(), String> {
    result.map_err(|err| format!("{}: {label} sync failed: {err}", path.display()))
}

pub fn create_temp_file(path: &Path, label: &str) -> Result<(std::path::PathBuf, File), String> {
    prepare_parent(path)?;
    let tmp = temp_path(path)?;
    let _ = fs::remove_file(&tmp);
    let file = create_file(&tmp, label)?;
    Ok((tmp, file))
}

pub fn finish_temp_file(tmp: &Path, path: &Path, label: &str) -> Result<(), String> {
    reject_symlink_components(tmp)?;
    if let Ok(meta) = fs::symlink_metadata(path)
        && meta.file_type().is_symlink()
    {
        let _ = fs::remove_file(tmp);
        return Err(format!("{}: output path uses symlink", path.display()));
    }
    fs::rename(tmp, path).map_err(|err| format!("{}: {label} rename failed: {err}", path.display()))
}

fn temp_path(path: &Path) -> Result<std::path::PathBuf, String> {
    let name = path
        .file_name()
        .ok_or_else(|| format!("{}: output path lacks file name", path.display()))?
        .to_string_lossy();
    let parent = path.parent().unwrap_or(Path::new(""));
    Ok(parent.join(format!(".{name}.tmp-{}", std::process::id())))
}

pub(crate) fn reject_existing_symlink_components(path: &Path) -> Result<(), String> {
    let mut cursor = start_cursor(path)?;
    for part in path.components() {
        match part {
            Component::RootDir | Component::CurDir => continue,
            Component::ParentDir | Component::Prefix(_) => {
                return Err(format!("{}: unsafe output path", path.display()));
            }
            Component::Normal(name) => cursor.push(name),
        }
        let Ok(meta) = fs::symlink_metadata(&cursor) else {
            break;
        };
        if meta.file_type().is_symlink() {
            return Err(format!("{}: output path uses symlink", path.display()));
        }
    }
    Ok(())
}

pub(crate) fn reject_symlink_components(path: &Path) -> Result<(), String> {
    let mut cursor = start_cursor(path)?;
    for part in path.components() {
        match part {
            Component::RootDir | Component::CurDir => continue,
            Component::ParentDir | Component::Prefix(_) => {
                return Err(format!("{}: unsafe output path", path.display()));
            }
            Component::Normal(name) => cursor.push(name),
        }
        if fs::symlink_metadata(&cursor)
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err(format!("{}: output path uses symlink", path.display()));
        }
    }
    Ok(())
}

fn start_cursor(path: &Path) -> Result<std::path::PathBuf, String> {
    start_cursor_with(path, std::env::current_dir)
}

pub(crate) fn start_cursor_with(
    path: &Path,
    current_dir: impl FnOnce() -> std::io::Result<std::path::PathBuf>,
) -> Result<std::path::PathBuf, String> {
    if path.is_absolute() {
        Ok(Path::new("/").to_path_buf())
    } else {
        current_dir().map_err(|err| format!("cwd unavailable: {err}"))
    }
}
