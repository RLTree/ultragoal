const CONTEXT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CANDIDATE: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SUBSTITUTE: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const WRONG_HOME: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const WRONG_TREE: &str = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const R3_CONTEXT: &str = "sha256:94705c615b06f6c59c08a3234713d0cf6c57b12d557bffe8121e7a8d18ae5168";
const R3_CANDIDATE: &str =
    "sha256:2f1c22491962bd8f3211d4b71709f98406c318b42bcacc48e8c9ff5fb2773ce1";
const R3_RESULT_PATH: &str =
    "docs/ultragoal-successor-live/worker-results/SUPPORTED-PACKAGE-IDENTITY-074.json";
const R3_WORK_PACKAGE_PATH: &str =
    "docs/ultragoal-successor-live/work-packages/SUPPORTED-PACKAGE-IDENTITY-074-R3.json";
const R3_WORK_PACKAGE_SHA256: &str =
    "sha256:a786a53849d5503b76908c6e0e2a4ec1a421be8f68d0180934bbe3746149c6b0";
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
struct Sink(Option<Vec<u8>>);

impl PackageEffects for Sink {
    fn read_package(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.0.clone())
    }

    fn compare_exchange_package(
        &mut self,
        expected: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()> {
        if self.0.as_deref().map(digest).as_deref() != expected {
            return Ok(false);
        }
        self.0 = replacement.map(<[u8]>::to_vec);
        Ok(true)
    }
}

#[derive(Default)]
struct Installed(Option<Vec<u8>>);

impl InstallEffects for Installed {
    fn read_installed(&mut self, _: &str, _: usize) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.0.clone())
    }

    fn compare_exchange_installed(
        &mut self,
        _: &str,
        expected: &ExpectedPrior,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()> {
        let matches = match expected {
            ExpectedPrior::Absent => self.0.is_none(),
            ExpectedPrior::ExactDigest(expected) => {
                self.0.as_deref().map(digest).as_deref() == Some(expected)
            }
        };
        if matches {
            self.0 = replacement.map(<[u8]>::to_vec);
        }
        Ok(matches)
    }
}

struct Fixture(PathBuf);

impl Fixture {
    fn new(label: &str) -> Self {
        let base = std::env::var_os("HUL_SUPPORTED_PACKAGE_SCRATCH_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let root = base.join(format!(
            "archive-{label}-{}-{}",
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
        let runtime = root.join("runtime-probe.sh");
        fs::write(
            &runtime,
            br##"#!/bin/sh
candidate="${1:-$HUL_CANDIDATE_ID}"
printf '%s\n' "HUL_RUNTIME_OBSERVATION={\"schema\":\"harness-ultragoal.runtime-probe.v1\",\"context_id\":\"$HUL_CONTEXT_ID\",\"candidate_id\":\"$candidate\",\"plugin_id\":\"$HUL_PLUGIN_ID\",\"version\":\"$HUL_VERSION\",\"package_sha256\":\"$HUL_PACKAGE_SHA256\",\"installed_tree_sha256\":\"$HUL_TREE_SHA256\",\"home_id\":\"$HUL_HOME_ID\",\"project_id\":\"$HUL_PROJECT_ID\",\"host_id\":\"$HUL_HOST_ID\",\"capability_sha256\":\"$HUL_CAPABILITY_SHA256\",\"binding_sha256\":\"$HUL_BINDING_SHA256\",\"executable_sha256\":\"$HUL_EXECUTABLE_SHA256\",\"session_nonce\":\"$HUL_SESSION_NONCE\"}"
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
