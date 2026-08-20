use super::*;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

struct TestRoot(PathBuf);

impl TestRoot {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "ultragoal-routine-device-neutral-{name}-{}-{}",
            std::process::id(),
            now_tick().expect("clock")
        ));
        fs::create_dir(&path).expect("create test root");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("chmod test root");
        Self(path)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn legacy_bytes(root: &Path, replacement_device: u64, replacement_inode: Option<u64>) -> Vec<u8> {
    let store = Store::open(root).expect("store");
    let key_file = store.open_existing(KEY_NAME, libc::O_RDONLY).expect("key");
    let key = read_key(&key_file).expect("key bytes");
    let key_id = sha256(key.bytes());
    let lock = store.open_existing(LOCK_NAME, libc::O_RDWR).expect("lock");
    let lock_identity = store
        .exact_identity(LOCK_NAME, &lock, 0o600)
        .expect("lock identity");
    let current = decode(
        &store.read_state().expect("state"),
        &key,
        &authority_id(&key_id, store.identity, lock_identity).expect("authority id"),
        &key_id,
        store.identity,
        lock_identity,
    )
    .expect("current payload");
    let root_identity = RootIdentity {
        device: replacement_device,
        inode: replacement_inode.unwrap_or(store.identity.inode),
        owner: store.identity.owner,
        mode: store.identity.mode,
    };
    let lock_identity = FileIdentity {
        device: replacement_device,
        ..lock_identity
    };
    let payload = LegacyPayloadV4 {
        schema_version: LEGACY_SCHEMA.to_owned(),
        authority_id: legacy_authority_id(&key_id, root_identity, lock_identity)
            .expect("legacy authority id"),
        key_id,
        root_identity,
        lock_identity,
        generation: current.generation,
        previous_head_sha256: current.previous_head_sha256,
        last_tick: current.last_tick,
        attempts: current.attempts,
        effects: current.effects,
        consumed_grants: current.consumed_grants,
    };
    let envelope = LegacyEnvelopeV4 {
        hmac_sha256: hmac(key.bytes(), &canonical(&payload).expect("legacy payload"))
            .expect("hmac"),
        payload,
    };
    canonical(&envelope).expect("legacy envelope")
}

fn replace_state(root: &Path, bytes: &[u8]) {
    let state = root.join(STATE_NAME);
    fs::write(&state, bytes).expect("replace state");
    fs::set_permissions(&state, fs::Permissions::from_mode(0o600)).expect("chmod state");
}

#[test]
fn authenticated_v4_device_renumbering_migrates_on_first_mutation() {
    let root = TestRoot::new("migrate");
    let _ = FileLedger::open_or_initialize(&root.0).expect("initialize");
    let current_device = fs::metadata(&root.0).expect("root metadata").dev();
    let legacy = legacy_bytes(&root.0, current_device + 1, None);
    let legacy_sha256 = sha256(&legacy);
    replace_state(&root.0, &legacy);

    let (ledger, mut head) = FileLedger::open_or_initialize(&root.0).expect("open legacy");
    assert_eq!(
        fs::read(root.0.join(STATE_NAME)).expect("zero-write legacy read"),
        legacy
    );
    ledger
        .with_payload(&mut head, true, |_payload, _tick| Ok(()))
        .expect("migrate on mutation");

    let migrated = fs::read(root.0.join(STATE_NAME)).expect("migrated state");
    let value: serde_json::Value = serde_json::from_slice(&migrated).expect("migrated json");
    assert_eq!(value["payload"]["schema_version"], SCHEMA);
    assert_eq!(value["payload"]["previous_head_sha256"], legacy_sha256);
    assert!(value["payload"]["root_identity"].get("device").is_none());
    assert!(value["payload"]["lock_identity"].get("device").is_none());
    FileLedger::open_existing(&root.0).expect("reopen migrated state");
}

#[test]
fn legacy_durable_inode_substitution_still_fails_closed() {
    let root = TestRoot::new("inode");
    let _ = FileLedger::open_or_initialize(&root.0).expect("initialize");
    let current = fs::metadata(&root.0).expect("root metadata");
    let legacy = legacy_bytes(&root.0, current.dev() + 1, Some(current.ino() + 1));
    replace_state(&root.0, &legacy);
    assert!(FileLedger::open_existing(&root.0).is_err());
}

#[test]
fn legacy_hmac_tampering_still_fails_closed() {
    let root = TestRoot::new("hmac");
    let _ = FileLedger::open_or_initialize(&root.0).expect("initialize");
    let current_device = fs::metadata(&root.0).expect("root metadata").dev();
    let mut legacy = legacy_bytes(&root.0, current_device + 1, None);
    let index = legacy.len() / 2;
    legacy[index] ^= 1;
    replace_state(&root.0, &legacy);
    assert!(FileLedger::open_existing(&root.0).is_err());
}
