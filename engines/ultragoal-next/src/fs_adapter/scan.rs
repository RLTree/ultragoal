use super::*;

/// A missed telemetry sample does not make a tiny directory unsafe to list.
/// Keep the unknown window small, then name pressure until observation returns.
fn listing_pressure(scanned: usize, headroom: Option<crate::resources::Headroom>) -> bool {
    match headroom {
        Some(host) => !host.listing_can_grow(scanned.saturating_add(1024)),
        None => scanned >= 4096,
    }
}
use crate::relative;
use std::ffi::CString;

impl Root {
    /// Enumerates names and kinds only. No regular-file content descriptor is opened.
    pub fn enumerate(&self, limits: &Limits) -> Listing {
        self.enumerate_excluding(limits, &[])
    }

    /// Exact raw-byte basenames are omitted at every depth before entry accounting or descent.
    pub fn enumerate_excluding(&self, limits: &Limits, excluded_basenames: &[Vec<u8>]) -> Listing {
        let limits = limits.bounded();
        let root_identity = Identity::of(&self.directory).ok();
        let valid = valid_exclusions(excluded_basenames);
        let mut excluded = if valid {
            excluded_basenames.to_vec()
        } else {
            Vec::new()
        };
        excluded.sort();
        excluded.dedup();
        let mut out = Listing {
            root_identity,
            excluded_basenames: excluded.clone(),
            members: Vec::new(),
            issues: Vec::new(),
            complete: true,
            scanned: 0,
        };
        if root_identity.is_none() {
            out.issue(&[], Problem::Unavailable(libc::EIO));
        }
        if !valid {
            out.issue(&[], Problem::InvalidPath);
            return out;
        }
        match self.directory.try_clone() {
            Ok(directory) => self.scan_directory(&[], 0, &limits, &excluded, &mut out, directory),
            Err(error) => out.issue(&[], Problem::Unavailable(code(error))),
        }
        out.members.sort_by(|a, b| a.path.cmp(&b.path));
        out.issues.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }

    fn scan_directory(
        &self,
        path: &[u8],
        depth: usize,
        limits: &Limits,
        excluded: &[Vec<u8>],
        out: &mut Listing,
        directory: File,
    ) {
        let before = match Identity::of(&directory) {
            Ok(id) => id,
            Err(error) => return out.issue(path, Problem::Unavailable(code(error))),
        };
        let mut stream = match os::DirStream::new(&directory) {
            Ok(stream) => stream,
            Err(error) => return out.issue(path, Problem::Unavailable(code(error))),
        };
        let mut children = Vec::new();
        loop {
            let name = match stream.next_name() {
                Ok(Some(name)) => name,
                Ok(None) => break,
                Err(error) => {
                    out.issue(path, Problem::Unavailable(code(error)));
                    break;
                }
            };
            if excluded.binary_search(&name).is_ok() {
                continue;
            }
            if out.scanned % 1024 == 0 && listing_pressure(out.scanned, crate::resources::current()) {
                out.issue(path, Problem::ResourcePressure);
                break;
            }
            if out.scanned >= limits.max_entries {
                out.issue(path, Problem::EntryBound);
                break;
            }
            out.scanned += 1;
            let child = relative::append(path, &name);
            if child.len() > limits.max_path_bytes {
                out.issue(path, Problem::Limit);
                continue;
            }
            let name = match CString::new(name) {
                Ok(name) => name,
                Err(_) => {
                    out.issue(&child, Problem::InvalidPath);
                    continue;
                }
            };
            let stat = match os::entry_stat(&directory, &name) {
                Ok(stat) => stat,
                Err(error) => {
                    out.issue(&child, Problem::Unavailable(code(error)));
                    continue;
                }
            };
            let kind = match stat.st_mode & libc::S_IFMT {
                libc::S_IFREG => Kind::File,
                libc::S_IFDIR => Kind::Directory,
                libc::S_IFLNK => Kind::Symlink,
                _ => Kind::Other,
            };
            out.members.push(Member {
                path: child.clone(),
                kind,
                identity: Identity::from_stat(&stat),
            });
            if kind == Kind::Directory {
                children.push((child, name));
            }
        }
        if let Err(error) = stream.close() {
            out.issue(path, Problem::Unavailable(code(error)));
        }
        if Identity::of(&directory).ok() != Some(before)
            || self
                .walk_directory(path)
                .ok()
                .and_then(|f| Identity::of(&f).ok())
                != Some(before)
            || (path.is_empty()
                && os::open_root(&self.path)
                    .ok()
                    .and_then(|f| Identity::of(&f).ok())
                    != Some(before))
        {
            out.issue(path, Problem::Unstable);
        }
        for (child, name) in children {
            if depth >= limits.max_depth {
                out.issue(&child, Problem::Limit);
            } else {
                match os::open_dir(&directory, &name) {
                    Ok(child_directory) => self.scan_directory(&child, depth + 1, limits, excluded, out, child_directory),
                    Err(error) => out.issue(&child, Problem::Unavailable(code(error))),
                }
            }
        }
    }

    pub(super) fn walk_directory(&self, path: &[u8]) -> io::Result<File> {
        let mut current = self.directory.try_clone()?;
        if !path.is_empty() {
            let parts = relative::components(path)
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
            for part in parts {
                current = os::open_dir(&current, &part)?;
            }
        }
        Ok(current)
    }
}

impl Listing {
    fn issue(&mut self, path: &[u8], problem: Problem) {
        self.complete = false;
        self.issues.push(Issue {
            path: path.to_vec(),
            problem,
        });
    }
}

#[cfg(test)]
mod pressure_tests {
    use super::*;

    #[test]
    fn unknown_telemetry_allows_small_scan_then_names_pressure() {
        let low = crate::resources::Headroom { physical: 16 << 30, available: 1 << 30 };
        let recovered = crate::resources::Headroom { physical: 16 << 30, available: 12 << 30 };
        assert!(!listing_pressure(0, None));
        assert!(!listing_pressure(3072, None));
        assert!(listing_pressure(4096, None));
        assert!(listing_pressure(0, Some(low)));
        assert!(!listing_pressure(4096, Some(recovered)));
    }
}
