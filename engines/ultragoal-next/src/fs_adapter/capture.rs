use super::*;
#[cfg(test)]
use crate::relative;
use std::ffi::CString;
use std::io::Read;

#[derive(Clone, Copy)]
pub(super) enum Phase {
    AfterTypeCheck,
    AfterOpen,
    AfterChunk,
    BeforeRevalidate,
}

const CHUNK: usize = 64 * 1024;

impl Root {
    #[cfg(test)]
    pub(super) fn capture_with_hook<F>(
        &self,
        path: &[u8],
        max_path: usize,
        cap: usize,
        mut hook: F,
    ) -> Capture
    where
        F: FnMut(Phase, usize),
    {
        if path.len() > max_path {
            return Capture::reject(path, Problem::Limit);
        }
        let parts = match relative::components(path) {
            Ok(parts) => parts,
            Err(()) => return Capture::reject(path, Problem::InvalidPath),
        };
        let (leaf, parent) = parts.split_last().expect("validated nonempty path");
        self.capture_window(parent, &[(path, leaf.clone())], cap, &mut hook)
            .pop()
            .expect("one capture per path")
    }

    /// Opens the held root's lineage down to `parent`, checking that the root path
    /// still names the held root first.
    pub(super) fn lineage(&self, parent: &[CString]) -> Result<(File, Vec<Identity>), Problem> {
        let mut directory = self
            .directory
            .try_clone()
            .map_err(|error| Problem::Unavailable(code(error)))?;
        let mut lineage =
            vec![Identity::of(&directory).map_err(|error| Problem::Unavailable(code(error)))?];
        if os::open_root(&self.path)
            .ok()
            .and_then(|f| Identity::of(&f).ok())
            != Some(lineage[0])
        {
            return Err(Problem::Unstable);
        }
        for part in parent {
            directory =
                os::open_dir(&directory, part).map_err(|error| Problem::Unavailable(code(error)))?;
            lineage.push(Identity::of(&directory).map_err(|error| Problem::Unavailable(code(error)))?);
        }
        Ok((directory, lineage))
    }

    /// Captures files that share one parent directory. The lineage is opened and
    /// checked once per window; each file costs fstatat, openat, fstat, its reads and
    /// close. After the window the lineage is walked again from the held root and each
    /// name must still bind the object read with an unchanged identity (every write
    /// changes ctime), and the root path must still name the held root.
    pub(super) fn capture_window<F>(
        &self,
        parent: &[CString],
        files: &[(&[u8], CString)],
        cap: usize,
        hook: &mut F,
    ) -> Vec<Capture>
    where
        F: FnMut(Phase, usize),
    {
        let (directory, lineage) = match self.lineage(parent) {
            Ok(value) => value,
            Err(problem) => {
                return files
                    .iter()
                    .map(|(path, _)| Capture::reject(path, problem))
                    .collect();
            }
        };
        let mut captures: Vec<Capture> = files
            .iter()
            .map(|(path, leaf)| read_leaf(&directory, path, leaf, cap, hook))
            .collect();
        hook(Phase::BeforeRevalidate, 0);
        let fresh = self
            .lineage(parent)
            .ok()
            .filter(|(_, ids)| *ids == lineage)
            .map(|(directory, _)| directory);
        for (capture, (path, leaf)) in captures.iter_mut().zip(files) {
            if capture.problem.is_some() {
                continue;
            }
            let bound = fresh
                .as_ref()
                .and_then(|d| os::entry_stat(d, leaf).ok())
                .map(|stat| Identity::from_stat(&stat));
            if bound != capture.identity {
                *capture = Capture::reject(path, Problem::Unstable);
            }
        }
        if os::open_root(&self.path)
            .ok()
            .and_then(|f| Identity::of(&f).ok())
            != Some(lineage[0])
        {
            for (capture, (path, _)) in captures.iter_mut().zip(files) {
                if capture.problem.is_none() {
                    *capture = Capture::reject(path, Problem::Unstable);
                }
            }
        }
        captures
    }

    /// Identity of each file without reading content, under the same window lineage
    /// checks; bytes are left empty. For revalidating an earlier capture.
    pub(super) fn identity_window(&self, parent: &[CString], files: &[(&[u8], CString)]) -> Vec<Capture> {
        let (directory, _) = match self.lineage(parent) {
            Ok(value) => value,
            Err(problem) => {
                return files
                    .iter()
                    .map(|(path, _)| Capture::reject(path, problem))
                    .collect();
            }
        };
        files
            .iter()
            .map(|(path, leaf)| match os::entry_stat(&directory, leaf) {
                Ok(stat) => {
                    let identity = Identity::from_stat(&stat);
                    if identity.mode & u32::from(libc::S_IFMT) != u32::from(libc::S_IFREG)
                        || identity.links != 1
                    {
                        Capture::reject(path, Problem::Unsupported)
                    } else {
                        Capture {
                            path: path.to_vec(),
                            bytes: Some(Vec::new()),
                            identity: Some(identity),
                            problem: None,
                        }
                    }
                }
                Err(error) => Capture::reject(path, Problem::Unavailable(code(error))),
            })
            .collect()
    }
}

fn read_leaf<F>(directory: &File, path: &[u8], leaf: &CString, cap: usize, hook: &mut F) -> Capture
where
    F: FnMut(Phase, usize),
{
    let mode = match os::entry_stat(directory, leaf) {
        Ok(stat) => stat.st_mode,
        Err(error) => return Capture::reject(path, Problem::Unavailable(code(error))),
    };
    if mode & libc::S_IFMT != libc::S_IFREG {
        return Capture::reject(path, Problem::Unsupported);
    }
    hook(Phase::AfterTypeCheck, 0);
    let mut file = match os::open_regular(directory, leaf) {
        Ok(file) => file,
        Err(error) => return Capture::reject(path, Problem::Unavailable(code(error))),
    };
    let before = match Identity::of(&file) {
        Ok(id) => id,
        Err(error) => return Capture::reject(path, Problem::Unavailable(code(error))),
    };
    if before.mode & u32::from(libc::S_IFMT) != u32::from(libc::S_IFREG) || before.links != 1 {
        return Capture::reject(path, Problem::Unsupported);
    }
    if before.size > cap as u64 {
        return Capture::reject(path, Problem::Limit);
    }
    hook(Phase::AfterOpen, 0);
    let mut bytes = Vec::new();
    // Sized for the observed length plus one byte, so a single short read ends a
    // file that did not grow; growth is read on and rejected below.
    if bytes.try_reserve_exact(before.size as usize + 1).is_err() {
        return Capture::reject(path, Problem::Limit);
    }
    let expected = before.size as usize;
    loop {
        let start = bytes.len();
        let want = if start <= expected { (expected + 1 - start).min(CHUNK) } else { CHUNK }
            .min(cap.saturating_sub(start).saturating_add(1));
        bytes.resize(start + want, 0);
        let count = match file.read(&mut bytes[start..]) {
            Ok(count) => count,
            Err(error) => return Capture::reject(path, Problem::Unavailable(code(error))),
        };
        bytes.truncate(start + count);
        if bytes.len() > cap {
            return Capture::reject(path, Problem::Limit);
        }
        if count > 0 {
            hook(Phase::AfterChunk, bytes.len());
        }
        if count < want {
            break;
        }
    }
    if bytes.len() as u64 != before.size {
        return Capture::reject(path, Problem::Unstable);
    }
    Capture {
        path: path.to_vec(),
        bytes: Some(bytes),
        identity: Some(before),
        problem: None,
    }
}
