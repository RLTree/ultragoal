use super::*;

#[cfg(target_vendor = "apple")]
mod darwin {
    use super::*;
    use crate::cli::successor_public::fit::set_quarantine_failpoint_for_test;
    use std::fs::File;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    struct Fixture {
        repo: Repository,
        home: PathBuf,
        state: PathBuf,
        authority: PathBuf,
        diagnosis_path: PathBuf,
        diagnosis: serde_json::Value,
        diagnosis_bytes: Vec<u8>,
        plan_id: String,
        quarantine_name: String,
    }

    struct TestStreams {
        exit_code: i32,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let repo = Repository::new(label);
            let home = repo.root.with_extension(format!("{label}-home"));
            let state = home.join(".codex/state/harness-ultragoal/repository-fit");
            let authority = state.join("authority");
            let pending = state.join("pending");
            fs::create_dir_all(&authority).unwrap();
            fs::create_dir_all(&pending).unwrap();
            for path in [
                &home,
                &home.join(".codex"),
                &home.join(".codex/state"),
                &home.join(".codex/state/harness-ultragoal"),
                &state,
                &authority,
                &pending,
            ] {
                fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
            }
            let home = fs::canonicalize(&home).unwrap();

            let plan = run(&repo, &home, &["--json", "fit", "plan"]);
            assert_eq!(plan.exit_code, 0);
            let plan_value: serde_json::Value = serde_json::from_slice(&plan.stdout).unwrap();
            let fit_plan_id = plan_value["plan"]["plan_sha256"].as_str().unwrap();
            let fit_plan_path = home.join("fit-plan.json");
            fs::write(&fit_plan_path, &plan.stdout).unwrap();
            fs::set_permissions(&fit_plan_path, fs::Permissions::from_mode(0o600)).unwrap();
            let applied = run(
                &repo,
                &home,
                &[
                    "--json",
                    "fit",
                    "apply",
                    "--plan",
                    fit_plan_path.to_str().unwrap(),
                    "--accept-plan",
                    fit_plan_id,
                ],
            );
            assert_eq!(applied.exit_code, 0);

            fs::remove_file(repo.root.join("migration/authority-routes.json")).unwrap();
            let current_device = fs::metadata(&authority).unwrap().dev();
            crate::repository_fit::simulate_device_drift_for_test(&authority, current_device + 1)
                .unwrap();
            let diagnosed = run(&repo, &home, &["--json", "diagnose"]);
            assert_eq!(diagnosed.exit_code, 1);
            let diagnosis: serde_json::Value = serde_json::from_slice(&diagnosed.stdout).unwrap();
            let diagnosis_bytes = diagnosed.stdout.strip_suffix(b"\n").unwrap().to_vec();
            assert_eq!(diagnosis["status"], "device_identity_changed");
            assert_eq!(
                diagnosis["quarantine_plan"]["apply_capability"],
                "fit_apply_exact_quarantine_plan"
            );
            let plan_id = diagnosis["quarantine_plan"]["plan_id"]
                .as_str()
                .unwrap()
                .to_owned();
            let quarantine_name = diagnosis["quarantine_plan"]["quarantine_owner"]
                .as_str()
                .unwrap()
                .to_owned();
            let diagnosis_path = home.join("repository-fit-quarantine-plan.json");
            fs::write(&diagnosis_path, &diagnosed.stdout).unwrap();
            fs::set_permissions(&diagnosis_path, fs::Permissions::from_mode(0o600)).unwrap();
            Self {
                repo,
                home,
                state,
                authority,
                diagnosis_path,
                diagnosis,
                diagnosis_bytes,
                plan_id,
                quarantine_name,
            }
        }

        fn apply(&self) -> TestStreams {
            run(
                &self.repo,
                &self.home,
                &[
                    "--json",
                    "fit",
                    "apply",
                    "--plan",
                    self.diagnosis_path.to_str().unwrap(),
                    "--accept-plan",
                    &self.plan_id,
                ],
            )
        }

        fn quarantine(&self) -> PathBuf {
            self.state.parent().unwrap().join(&self.quarantine_name)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            set_quarantine_failpoint_for_test(None);
            let _ = fs::remove_dir_all(&self.home);
        }
    }

    fn run(repo: &Repository, home: &Path, arguments: &[&str]) -> TestStreams {
        let ParseOutcome::Invocation(invocation) = parse_args(arguments.iter().copied()).unwrap()
        else {
            panic!("expected invocation")
        };
        let streams = execute_invocation_with_home(&repo.root, invocation, Some(home))
            .render(OutputMode::Json);
        TestStreams {
            exit_code: streams.exit_code,
            stdout: streams.stdout,
            stderr: streams.stderr,
        }
    }

    #[test]
    fn exact_diagnosis_record_quarantines_the_whole_owner_and_bootstraps_fresh_authority() {
        let fixture = Fixture::new("fit-authority-quarantine-success");
        let target_before = tree(&fixture.repo.root);
        let old_owner = tree(&fixture.state);
        let outcome = fixture.apply();
        assert_eq!(
            outcome.exit_code,
            0,
            "stdout={} stderr={}",
            String::from_utf8_lossy(&outcome.stdout),
            String::from_utf8_lossy(&outcome.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&outcome.stdout).unwrap();
        assert_eq!(
            value["schema_version"],
            "RepositoryFitAuthorityQuarantineOutcome-v1"
        );
        assert_eq!(value["status"], "quarantined_and_fresh_owner_bootstrapped");
        assert_eq!(value["plan_id"], fixture.plan_id);
        assert_eq!(value["effect"], "host_state_write");
        assert_eq!(value["effect_started"], true);
        assert_eq!(value["fresh_owner_bootstrapped"], true);
        assert_eq!(value["quarantine_retained"], true);
        assert_eq!(value["target_effect"], "none");
        assert_eq!(tree(&fixture.repo.root), target_before);
        assert_eq!(tree(&fixture.quarantine()), old_owner);
        assert_eq!(fs::read_dir(&fixture.authority).unwrap().count(), 3);

        let replay = fixture.apply();
        assert_eq!(replay.exit_code, 1);
        assert!(replay.stdout.is_empty());
        assert_eq!(tree(&fixture.repo.root), target_before);
        assert_eq!(tree(&fixture.quarantine()), old_owner);
    }

    #[test]
    fn failures_before_bootstrap_restore_the_exact_source_owner() {
        for failpoint in [
            "after_rename_before_parent_sync",
            "after_parent_sync_before_bootstrap",
        ] {
            let fixture = Fixture::new(failpoint);
            let target_before = tree(&fixture.repo.root);
            let owner_before = tree(&fixture.state);
            set_quarantine_failpoint_for_test(Some(failpoint));
            let outcome = fixture.apply();
            set_quarantine_failpoint_for_test(None);
            assert_eq!(outcome.exit_code, 1);
            let value: serde_json::Value = serde_json::from_slice(&outcome.stdout).unwrap();
            assert_eq!(value["status"], "rolled_back_before_bootstrap");
            assert_eq!(value["rollback_complete"], true);
            assert_eq!(value["quarantine_retained"], false);
            assert_eq!(tree(&fixture.state), owner_before);
            assert!(!fixture.quarantine().exists());
            assert_eq!(tree(&fixture.repo.root), target_before);
        }
    }

    #[test]
    fn failure_after_bootstrap_starts_retains_both_states_as_ambiguous() {
        let fixture = Fixture::new("fit-authority-quarantine-ambiguous");
        let target_before = tree(&fixture.repo.root);
        let old_owner = tree(&fixture.state);
        set_quarantine_failpoint_for_test(Some("after_fresh_owner_created"));
        let outcome = fixture.apply();
        set_quarantine_failpoint_for_test(None);
        assert_eq!(outcome.exit_code, 1);
        let value: serde_json::Value = serde_json::from_slice(&outcome.stdout).unwrap();
        assert_eq!(value["status"], "ambiguous_after_bootstrap_started");
        assert_eq!(value["rollback_complete"], false);
        assert_eq!(value["fresh_owner_bootstrapped"], false);
        assert_eq!(value["quarantine_retained"], true);
        assert_eq!(tree(&fixture.quarantine()), old_owner);
        assert!(fixture.state.is_dir());
        assert_eq!(tree(&fixture.repo.root), target_before);
    }

    #[test]
    fn stale_destination_and_parent_lock_contention_refuse_before_effect() {
        let fixture = Fixture::new("fit-authority-quarantine-preconditions");
        let target_before = tree(&fixture.repo.root);
        let owner_before = tree(&fixture.state);
        fs::create_dir(&fixture.quarantine()).unwrap();
        fs::set_permissions(fixture.quarantine(), fs::Permissions::from_mode(0o700)).unwrap();
        let destination_collision = fixture.apply();
        assert_eq!(destination_collision.exit_code, 1);
        assert!(fixture.state.is_dir());
        assert_eq!(tree(&fixture.state), owner_before);
        fs::remove_dir(&fixture.quarantine()).unwrap();

        let parent = File::open(fixture.state.parent().unwrap()).unwrap();
        // SAFETY: the test owns this descriptor and releases the advisory lock below.
        assert_eq!(unsafe { libc::flock(parent.as_raw_fd(), libc::LOCK_SH) }, 0);
        let contention = fixture.apply();
        // SAFETY: the test owns this descriptor and is finished with the lock.
        unsafe { libc::flock(parent.as_raw_fd(), libc::LOCK_UN) };
        assert_eq!(contention.exit_code, 1);
        assert!(fixture.state.is_dir());
        assert_eq!(tree(&fixture.state), owner_before);
        assert_eq!(tree(&fixture.repo.root), target_before);
    }

    #[test]
    fn record_substitution_and_source_inventory_drift_refuse_without_quarantine() {
        let fixture = Fixture::new("fit-authority-quarantine-stale");
        let target_before = tree(&fixture.repo.root);
        let owner_before = tree(&fixture.state);

        fs::write(
            &fixture.diagnosis_path,
            serde_json::to_vec_pretty(&fixture.diagnosis).unwrap(),
        )
        .unwrap();
        let noncanonical = fixture.apply();
        assert_eq!(noncanonical.exit_code, 2);
        assert_eq!(tree(&fixture.state), owner_before);
        assert!(!fixture.quarantine().exists());

        let canonical = fixture.diagnosis_bytes.clone();
        let marker = b"\"status\":\"device_identity_changed\",";
        let offset = canonical
            .windows(marker.len())
            .position(|window| window == marker)
            .unwrap();
        let mut duplicate = canonical.clone();
        duplicate.splice(offset..offset, marker.iter().copied());
        fs::write(&fixture.diagnosis_path, duplicate).unwrap();
        let duplicate_key = fixture.apply();
        assert_eq!(duplicate_key.exit_code, 2);
        assert_eq!(tree(&fixture.state), owner_before);
        assert!(!fixture.quarantine().exists());

        let substituted = String::from_utf8(fixture.diagnosis_bytes.clone())
            .unwrap()
            .replacen(
                "revalidate_complete_source_inventory_under_exclusive_parent_custody",
                "rewrite_selected_legacy_state",
                1,
            );
        fs::write(&fixture.diagnosis_path, substituted).unwrap();
        let rejected = fixture.apply();
        assert_eq!(rejected.exit_code, 1);
        assert_eq!(tree(&fixture.state), owner_before);
        assert!(!fixture.quarantine().exists());

        fs::write(&fixture.diagnosis_path, &fixture.diagnosis_bytes).unwrap();

        let parent = fixture.state.parent().unwrap().to_path_buf();
        let displaced_parent = parent.with_file_name("harness-ultragoal.displaced");
        fs::rename(&parent, &displaced_parent).unwrap();
        fs::create_dir(&parent).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        fs::rename(
            displaced_parent.join("repository-fit"),
            parent.join("repository-fit"),
        )
        .unwrap();
        let parent_substitution = fixture.apply();
        assert_eq!(parent_substitution.exit_code, 1);
        assert!(fixture.state.is_dir());
        assert!(!fixture.quarantine().exists());
        fs::rename(
            parent.join("repository-fit"),
            displaced_parent.join("repository-fit"),
        )
        .unwrap();
        fs::remove_dir(&parent).unwrap();
        fs::rename(&displaced_parent, &parent).unwrap();

        let ledger = fixture.authority.join("authority-ledger.json");
        let mut bytes = fs::read(&ledger).unwrap();
        bytes.push(b'\n');
        fs::write(&ledger, bytes).unwrap();
        let drifted = fixture.apply();
        assert_eq!(drifted.exit_code, 1);
        assert!(fixture.state.is_dir());
        assert!(!fixture.quarantine().exists());
        assert_eq!(tree(&fixture.repo.root), target_before);
    }
}
