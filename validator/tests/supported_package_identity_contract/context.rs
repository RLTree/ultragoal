const CONTEXT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CANDIDATE: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SUBSTITUTE: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const WRONG_HOME: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const WRONG_TREE: &str = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
static NEXT: AtomicU64 = AtomicU64::new(0);

fn snapshot_tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(root: &Path, current: &Path, rows: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries = fs::read_dir(current)
            .unwrap()
            .map(|row| row.unwrap())
            .collect::<Vec<_>>();
        entries.sort_by_key(|row| row.file_name());
        for entry in entries {
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                walk(root, &path, rows);
            } else {
                rows.push((
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows
}

fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[derive(Default)]
struct Sink(Option<Vec<u8>>);

impl PackageEffects for Sink {
    fn read_package(
        &mut self,
        _: usize,
    ) -> Result<Option<Vec<u8>>, ultragoal::distribution::EffectFailure> {
        Ok(self.0.clone())
    }

    fn compare_exchange_package(
        &mut self,
        expected: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ultragoal::distribution::EffectFailure> {
        if self.0.as_deref().map(digest).as_deref() != expected {
            return Ok(false);
        }
        self.0 = replacement.map(<[u8]>::to_vec);
        Ok(true)
    }
}

struct Fixture(PathBuf);

impl Fixture {
    fn new(label: &str) -> Self {
        let base = std::env::var_os("HUL_SUPPORTED_PACKAGE_SCRATCH_ROOT")
            .or_else(|| std::env::var_os("CODEX_WORKTREE_TMP"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        let root = base.join(format!(
            "hul-distribution-archive-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("source")).unwrap();
        fs::create_dir_all(root.join("home")).unwrap();
        fs::create_dir_all(root.join("project")).unwrap();
        let manifest = json!({
            "name":"harness-ultragoal",
            "version":"0.0.11",
            "description":"Repository fit, routine work, diagnosis, proof, and migration.",
            "author":{"name":"Terry Noblin","email":"tree@terrynoblin.dev","url":"https://terrynoblin.dev"},
            "homepage":"https://terrynoblin.dev/harness-ultragoal",
            "repository":"https://github.com/terrynoblin/harness-ultragoal",
            "license":"UNLICENSED",
            "keywords":["agent-first","verification"],
            "skills":"./skills/",
            "interface":{
                "displayName":"Harness Ultragoal",
                "shortDescription":"One evidence-bound front door.",
                "longDescription":"Route repository work through explicit authority and effects.",
                "developerName":"Terry Noblin",
                "category":"Productivity",
                "capabilities":["Read","Write"],
                "websiteURL":"https://terrynoblin.dev/harness-ultragoal",
                "defaultPrompt":["Classify this repository task and route it safely."],
                "brandColor":"#3B82F6"
            }
        });
        fs::write(
            root.join("source/plugin.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("source/skill-one.md"),
            b"---\nname: harness-ultragoal\ndescription: Harness front door\n---\n",
        )
        .unwrap();
        fs::write(
            root.join("source/skill-two.md"),
            b"---\nname: prove\ndescription: Proof workflow\n---\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("plugins/harness-ultragoal/runtime")).unwrap();
        let runtime = root.join("plugins/harness-ultragoal/runtime/runtime-probe-bin");
        fs::write(
            &runtime,
            br##"#!/bin/sh
if [ "$#" -ne 0 ]; then
  printf '%s\n' "HUL_RUNTIME_OBSERVATION={\"schema\":\"harness-ultragoal.runtime-probe.v1\",\"session_nonce\":\"stale\"}"
  exit 0
fi
printf '%s\n' "HUL_RUNTIME_OBSERVATION={\"schema\":\"harness-ultragoal.runtime-probe.v1\",\"session_nonce\":\"$HUL_SESSION_NONCE\"}"
"##,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&runtime, fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self(root)
    }

    fn plan(&self) -> PackagePlan {
        let entries = [
            json!({"path":".codex-plugin/plugin.json","source_path":"source/plugin.json","role":"manifest","executable":false}),
            json!({"path":"runtime/runtime-probe-bin","source_path":"plugins/harness-ultragoal/runtime/runtime-probe-bin","role":"executable","executable":true}),
            json!({"path":"skills/harness-ultragoal/SKILL.md","source_path":"source/skill-one.md","role":"skill","executable":false}),
            json!({"path":"skills/prove/SKILL.md","source_path":"source/skill-two.md","role":"skill","executable":false}),
        ];
        let spec = serde_json::to_vec(&json!({
            "schema":"harness-ultragoal.package-plan.v1",
            "context_id":CONTEXT,
            "candidate_id":CANDIDATE,
            "plugin_id":"harness-ultragoal",
            "version":"0.0.11",
            "source_date_epoch":1_700_000_000u64,
            "entries":entries
        }))
        .unwrap();
        plan_package(&self.0, &spec).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct EntryLayout {
    path: Range<usize>,
    mode: usize,
    role: usize,
    length: usize,
    digest: Range<usize>,
    payload: Range<usize>,
}

struct Layout {
    metadata: Vec<Range<usize>>,
    epoch: usize,
    count: usize,
    entries: Vec<EntryLayout>,
}
