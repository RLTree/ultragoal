use std::ffi::CString;

pub(crate) fn components(path: &[u8]) -> Result<Vec<CString>, ()> {
    if path.is_empty() || path[0] == b'/' || path.contains(&0) {
        return Err(());
    }
    path.split(|byte| *byte == b'/')
        .map(|part| {
            if part.is_empty() || part == b"." || part == b".." {
                Err(())
            } else {
                CString::new(part).map_err(|_| ())
            }
        })
        .collect()
}

pub(crate) fn append(parent: &[u8], name: &[u8]) -> Vec<u8> {
    let mut path = Vec::with_capacity(parent.len() + usize::from(!parent.is_empty()) + name.len());
    path.extend_from_slice(parent);
    if !parent.is_empty() {
        path.push(b'/');
    }
    path.extend_from_slice(name);
    path
}
