use super::*;

pub(crate) fn bounded_read(file: &mut File, maximum: usize) -> Result<Vec<u8>, FitError> {
    let mut bytes = Vec::new();
    Read::by_ref(file)
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error(FitErrorId::ReadFailed))?;
    if bytes.len() > maximum {
        return Err(error(FitErrorId::ResourceLimit));
    }
    Ok(bytes)
}

pub(crate) fn identity(metadata: &fs::Metadata) -> PathStat {
    PathStat {
        device: metadata.dev(),
        inode: metadata.ino(),
        links: metadata.nlink(),
        length: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
        regular: metadata.is_file(),
    }
}
