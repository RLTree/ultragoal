use super::*;
use crate::relative;

impl Root {
    /// `None` performs membership-only enumeration; `Some` reads only selected regular files.
    pub fn observe(&self, selected: Option<&[Vec<u8>]>, limits: &Limits) -> Snapshot {
        self.observe_with_hook(selected, limits, |_| {})
    }

    pub fn observe_excluding(
        &self,
        selected: Option<&[Vec<u8>]>,
        limits: &Limits,
        excluded_basenames: &[Vec<u8>],
    ) -> Snapshot {
        self.observe_with_hook_excluding(selected, limits, excluded_basenames, |_| {})
    }

    pub(super) fn observe_with_hook<F>(
        &self,
        selected: Option<&[Vec<u8>]>,
        limits: &Limits,
        after_list: F,
    ) -> Snapshot
    where
        F: FnOnce(&Listing),
    {
        self.observe_with_hook_excluding(selected, limits, &[], after_list)
    }

    fn observe_with_hook_excluding<F>(
        &self,
        selected: Option<&[Vec<u8>]>,
        limits: &Limits,
        excluded_basenames: &[Vec<u8>],
        after_list: F,
    ) -> Snapshot
    where
        F: FnOnce(&Listing),
    {
        let limits = limits.bounded();
        let listing = self.enumerate_excluding(&limits, excluded_basenames);
        after_list(&listing);
        let mut captures = Vec::new();
        if let Some(paths) = selected {
            let truncated = paths.len() > limits.max_entries;
            let mut wanted: Vec<Vec<u8>> = paths
                .iter()
                .take(limits.max_entries.saturating_add(1))
                .map(|path| path[..path.len().min(HARD_MAX_PATH_BYTES + 1)].to_vec())
                .collect();
            wanted.sort();
            wanted.dedup();
            captures = self.capture_selected_excluding(&wanted, &limits, excluded_basenames);
            if truncated
                && captures
                    .iter()
                    .all(|item| item.path != b"" || item.problem != Some(Problem::Limit))
            {
                captures.push(Capture::reject(&[], Problem::Limit));
            }
            for capture in &mut captures {
                if let Some(actual) = capture.identity {
                    let listed = listing
                        .members
                        .binary_search_by(|member| member.path.cmp(&capture.path))
                        .ok()
                        .and_then(|index| listing.members.get(index));
                    if listed.map(|member| (member.kind, member.identity))
                        != Some((Kind::File, actual))
                    {
                        capture.bytes = None;
                        capture.identity = None;
                        capture.problem = Some(Problem::Unstable);
                    }
                }
            }
        }
        Snapshot { listing, captures }
    }

    /// Each result is independent; rejected captures expose no bytes.
    pub fn capture_selected(&self, paths: &[Vec<u8>], limits: &Limits) -> Vec<Capture> {
        self.capture_selected_excluding(paths, limits, &[])
    }

    pub fn capture_selected_excluding(
        &self,
        paths: &[Vec<u8>],
        limits: &Limits,
        excluded_basenames: &[Vec<u8>],
    ) -> Vec<Capture> {
        self.selected(paths, limits, excluded_basenames, true)
    }

    /// Current identities of selected files without reading content (bytes empty).
    pub fn identities_selected_excluding(
        &self,
        paths: &[Vec<u8>],
        limits: &Limits,
        excluded_basenames: &[Vec<u8>],
    ) -> Vec<Capture> {
        self.selected(paths, limits, excluded_basenames, false)
    }

    fn selected(
        &self,
        paths: &[Vec<u8>],
        limits: &Limits,
        excluded_basenames: &[Vec<u8>],
        read: bool,
    ) -> Vec<Capture> {
        let limits = limits.bounded();
        let valid = valid_exclusions(excluded_basenames);
        let checked = |path: &Vec<u8>| -> Result<Vec<std::ffi::CString>, Capture> {
            if !valid {
                return Err(Capture::reject(path, Problem::InvalidPath));
            }
            if path.len() > limits.max_path_bytes {
                return Err(Capture::reject(path, Problem::Limit));
            }
            let parts = match relative::components(path) {
                Ok(parts) => parts,
                Err(_) => return Err(Capture::reject(path, Problem::InvalidPath)),
            };
            if parts.iter().any(|part| {
                excluded_basenames
                    .iter()
                    .any(|name| name == part.as_bytes())
            }) {
                return Err(Capture::reject(path, Problem::Excluded));
            }
            Ok(parts)
        };
        // Paths are read concurrently in windows, each file capped at the budget left
        // when the window starts; the total budget is then applied in path order,
        // which accepts and rejects exactly the files a serial pass would. Within a
        // window, consecutive files of one directory share one lineage check.
        const CAPTURE_WINDOW: usize = 256;
        const CAPTURE_THREADS: usize = 8;
        enum Job<'a> {
            Rejected(Capture),
            Directory(Vec<std::ffi::CString>, Vec<(&'a [u8], std::ffi::CString)>),
        }
        let selected = &paths[..paths.len().min(limits.max_entries)];
        let mut used = 0usize;
        let mut captures = Vec::with_capacity(selected.len() + 1);
        for window in selected.chunks(CAPTURE_WINDOW) {
            let cap = limits
                .max_file_bytes
                .min(limits.max_total_bytes.saturating_sub(used));
            let mut jobs: Vec<Job> = Vec::new();
            for path in window {
                match checked(path) {
                    Err(capture) => jobs.push(Job::Rejected(capture)),
                    Ok(mut parts) => {
                        let leaf = parts.pop().expect("validated nonempty path");
                        match jobs.last_mut() {
                            Some(Job::Directory(parent, files)) if *parent == parts => {
                                files.push((path.as_slice(), leaf))
                            }
                            _ => jobs.push(Job::Directory(parts, vec![(path.as_slice(), leaf)])),
                        }
                    }
                }
            }
            let run = |job: &Job| -> Vec<Capture> {
                match job {
                    Job::Rejected(capture) => vec![Capture {
                        path: capture.path.clone(),
                        bytes: None,
                        identity: None,
                        problem: capture.problem,
                    }],
                    Job::Directory(parent, files) if read => {
                        self.capture_window(parent, files, cap, &mut |_, _| {})
                    }
                    Job::Directory(parent, files) => self.identity_window(parent, files),
                }
            };
            let per = window.len().div_ceil(CAPTURE_THREADS).max(1);
            let mut groups: Vec<&[Job]> = Vec::new();
            let (mut start, mut count) = (0usize, 0usize);
            for (i, job) in jobs.iter().enumerate() {
                count += match job {
                    Job::Rejected(_) => 1,
                    Job::Directory(_, files) => files.len(),
                };
                if count >= per {
                    groups.push(&jobs[start..=i]);
                    (start, count) = (i + 1, 0);
                }
            }
            if start < jobs.len() {
                groups.push(&jobs[start..]);
            }
            let results: Vec<Capture> = std::thread::scope(|scope| {
                let handles: Vec<_> = groups
                    .iter()
                    .map(|group| scope.spawn(|| group.iter().flat_map(run).collect::<Vec<_>>()))
                    .collect();
                handles
                    .into_iter()
                    .flat_map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
                    .collect()
            });
            for (path, result) in window.iter().zip(results) {
                let len = result.bytes.as_ref().map(Vec::len);
                match len {
                    Some(len) if len > limits.max_total_bytes.saturating_sub(used) => {
                        captures.push(Capture::reject(path, Problem::Limit));
                    }
                    Some(len) => {
                        used += len;
                        captures.push(result);
                    }
                    None => captures.push(result),
                }
            }
        }
        if paths.len() > limits.max_entries {
            captures.push(Capture::reject(&[], Problem::Limit));
        }
        captures
    }
}
