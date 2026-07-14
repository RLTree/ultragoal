#[cfg(all(test, unix))]
mod restoration_tests {
    use super::*;
    use crate::distribution::{
        EffectPoint, PackagePlan, PackageSnapshot, ScopedFile, assert_test_effect_hook_consumed,
        build_package, plan_package, set_test_effect_hook_matching,
    };
    use crate::plugin_product::lifecycle::{
        LifecycleAuthorization, LifecycleIntent, LifecycleRequest, PackageAuthority, Version, plan,
    };
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    struct Bundle {
        plan: PackagePlan,
        snapshot: PackageSnapshot,
        authority: PackageAuthority,
    }

    #[test]
    fn restore_refusal_requeues_installed_mutation_for_same_operation_retry() {
        let root = std::env::var_os("CODEX_WORKTREE_TMP")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                "hul-distribution-adapter-refusal-{}-{}",
                std::process::id(),
                NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&root).unwrap();
        let v11 = bundle(&root, "0.0.11");
        let v12 = bundle(&root, "0.0.12");
        write_surface(&root, INSTALLED_PATH, v11.snapshot.archive());
        write_surface(&root, CACHE_PATH, v11.snapshot.archive());
        let before = state(&v11.authority, 8);
        let request = LifecycleRequest {
            intent: LifecycleIntent::MonotonicUpdate,
            target: Some(v12.authority.clone()),
            prior_authority: None,
            authorization: LifecycleAuthorization {
                allow_host_write: true,
                allow_downgrade: false,
                expected_installed_sha256: Some(v11.authority.package_sha256.clone()),
            },
        };
        let lifecycle = plan(&before, &request).unwrap();
        let package = BoundPackage::bind(&v12.plan, &v12.snapshot, &lifecycle).unwrap();
        let mut effects = Effects::bind(
            confined(&root),
            package,
            lifecycle.before.clone(),
            lifecycle.expected_after.clone(),
            lifecycle.effects.clone(),
        )
        .unwrap();
        effects
            .execute(LifecycleEffect::InstallPackage, &lifecycle.expected_after)
            .unwrap();
        effects
            .execute(LifecycleEffect::RefreshCache, &lifecycle.expected_after)
            .unwrap();
        assert_eq!(effects.mutation_count(), 2);

        let collision = Arc::new(Mutex::new(None::<String>));
        let collision_for_hook = Arc::clone(&collision);
        let root_for_hook = root.clone();
        set_test_effect_hook_matching(EffectPoint::Rename, ".hul-stage-", move |detail| {
            let stage = detail
                .split("<->")
                .chain(detail.split("->"))
                .find(|row| row.contains(".hul-stage-"))
                .unwrap()
                .to_owned();
            let stage = stage
                .rsplit_once('/')
                .map_or(stage.as_str(), |(_, name)| name)
                .to_owned();
            let stage_path = root_for_hook.join(&stage);
            assert!(stage_path.exists(), "missing stage for detail {detail}");
            std::fs::remove_file(&stage_path).unwrap();
            collision_for_hook.lock().unwrap().replace(stage);
        });

        let failure = effects.restore_inner(&before).unwrap_err();
        assert_eq!(failure.id(), AdapterErrorId::DistributionEffect);
        assert_test_effect_hook_consumed();
        assert_eq!(effects.mutation_count(), 2);
        let stage = collision.lock().unwrap().take().unwrap();
        assert!(!root.join(stage).exists());

        effects.restore_inner(&before).unwrap();
        assert_eq!(effects.mutation_count(), 0);
        assert_eq!(effects.observed_state().unwrap(), before);
        let _ = std::fs::remove_dir_all(root);
    }

    fn bundle(root: &std::path::Path, version: &str) -> Bundle {
        let label = version.replace('.', "-");
        let source = root.join(format!("source-{label}"));
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("plugin.json"), manifest(version).to_string()).unwrap();
        std::fs::write(
            source.join("front.md"),
            b"---\nname: harness-ultragoal\n---\n",
        )
        .unwrap();
        let spec = json!({
            "schema":"harness-ultragoal.package-plan.v1",
            "context_id":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "candidate_id":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "plugin_id":"harness-ultragoal",
            "version":version,
            "source_date_epoch":1_700_000_000u64,
            "entries":[
                {"path":".codex-plugin/plugin.json","source_path":format!("source-{label}/plugin.json"),"role":"manifest","executable":false},
                {"path":"skills/harness-ultragoal/SKILL.md","source_path":format!("source-{label}/front.md"),"role":"skill","executable":false}
            ]
        });
        let plan = plan_package(root, &serde_json::to_vec(&spec).unwrap()).unwrap();
        let mut sink = ScopedFile::new(
            confined(root),
            &format!("packages/harness-ultragoal-{label}.hugpkg"),
        )
        .unwrap();
        let snapshot = build_package(&plan, &mut sink).unwrap();
        let authority = PackageAuthority {
            version: Version::parse(version).unwrap(),
            package_sha256: snapshot.package_sha256().to_owned(),
            inventory_sha256: snapshot.inventory_sha256().to_owned(),
            candidate_id: snapshot.candidate_id().to_owned(),
        };
        Bundle {
            plan,
            snapshot,
            authority,
        }
    }

    fn manifest(version: &str) -> serde_json::Value {
        json!({
            "name":"harness-ultragoal",
            "version":version,
            "description":"Repository fit, routine work, diagnosis, proof, and migration.",
            "author":{"name":"Terry Noblin","email":"tree@terrynoblin.dev"},
            "license":"UNLICENSED",
            "skills":"./skills/"
        })
    }

    fn write_surface(root: &std::path::Path, path: &str, bytes: &[u8]) {
        ScopedFile::new(confined(root), path)
            .unwrap()
            .apply(None, Some(bytes))
            .unwrap();
    }

    fn confined(root: &std::path::Path) -> ConfinedRoot {
        ConfinedRoot::open(root).unwrap()
    }

    fn state(authority: &PackageAuthority, generation: u64) -> LifecycleState {
        LifecycleState {
            installed: Some(authority.clone()),
            cache: Some(authority.clone()),
            generation,
            recovery_required: false,
        }
    }
}
