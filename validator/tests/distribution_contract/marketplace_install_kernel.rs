use crate::distribution::{
    CodexPlugin, DistributionErrorId as ErrorId, MarketplaceEffects, PackageIdentity,
    SourceIdentity, apply_marketplace, plan_codex_marketplace, rollback_marketplace,
};
use crate::support::{CANDIDATE_ID, CONTEXT_ID, digest};
use serde_json::{Value, json};

const A: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const C: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn package(version: &str, tree: &str, archive: &str) -> PackageIdentity {
    PackageIdentity::new(
        SourceIdentity::new(
            CONTEXT_ID.into(),
            CANDIDATE_ID.into(),
            "harness-ultragoal".into(),
            version.into(),
            A.into(),
            A.into(),
        )
        .unwrap(),
        tree.into(),
        archive.into(),
    )
    .unwrap()
}

fn existing_marketplace() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "name":"local-harness-plugins",
        "interface":{"displayName":"Local Harness Plugins"},
        "plugins":[{
            "name":"narrated-record-replay",
            "source":{"source":"local","path":"./plugins/narrated-record-replay"},
            "policy":{"installation":"AVAILABLE","authentication":"ON_INSTALL"},
            "category":"Productivity"
        }]
    }))
    .unwrap()
}

#[derive(Default)]
struct MarketplaceSink {
    bytes: Option<Vec<u8>>,
    race: Option<Vec<u8>>,
    corrupt_after_write: bool,
    fail_after_write: bool,
    transitions: usize,
}

impl MarketplaceEffects for MarketplaceSink {
    fn read(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        if self.fail_after_write && self.transitions > 0 {
            return Err(());
        }
        let mut bytes = self.bytes.clone();
        if self.corrupt_after_write && self.transitions > 0 {
            bytes.as_mut().unwrap().push(b' ');
        }
        Ok(bytes)
    }
    fn compare_exchange(
        &mut self,
        expected: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()> {
        self.transitions += 1;
        if let Some(race) = self.race.take() {
            self.bytes = Some(race);
        }
        if self.bytes.as_deref().map(digest).as_deref() != expected {
            return Ok(false);
        }
        self.bytes = replacement.map(<[u8]>::to_vec);
        Ok(true)
    }
}

#[test]
fn real_codex_marketplace_plan_preserves_unrelated_entries_and_rolls_back() {
    let prior = existing_marketplace();
    let plan = plan_codex_marketplace(
        Some(&prior),
        Some(&digest(&prior)),
        "local-harness-plugins",
        "Local Harness Plugins",
        CodexPlugin::harness_ultragoal(),
        package("0.0.12", B, C),
    )
    .unwrap();
    let debug = format!("{plan:?}");
    assert!(!debug.contains("narrated-record-replay"));
    assert!(!debug.contains("./plugins/harness-ultragoal"));
    let prior_value: Value = serde_json::from_slice(&prior).unwrap();
    let value: Value = serde_json::from_slice(plan.replacement()).unwrap();
    assert_eq!(value["plugins"][0], prior_value["plugins"][0]);
    assert_eq!(value["plugins"][0]["name"], "narrated-record-replay");
    assert_eq!(value["plugins"][1]["name"], "harness-ultragoal");
    assert_eq!(
        value["plugins"][1]["source"]["path"],
        "./plugins/harness-ultragoal"
    );
    let mut sink = MarketplaceSink {
        bytes: Some(prior.clone()),
        ..MarketplaceSink::default()
    };
    let transaction = apply_marketplace(&plan, &mut sink).unwrap();
    assert_eq!(sink.bytes.as_deref(), Some(plan.replacement()));
    rollback_marketplace(transaction, &mut sink).unwrap();
    assert_eq!(sink.bytes, Some(prior.clone()));

    let mut corrupt = MarketplaceSink {
        bytes: Some(prior.clone()),
        corrupt_after_write: true,
        ..MarketplaceSink::default()
    };
    assert_eq!(
        apply_marketplace(&plan, &mut corrupt).unwrap_err().id(),
        ErrorId::ArchiveMismatch
    );
    assert_eq!(corrupt.bytes, Some(prior));

    let mut failed_read = MarketplaceSink {
        bytes: Some(existing_marketplace()),
        fail_after_write: true,
        ..MarketplaceSink::default()
    };
    assert_eq!(
        apply_marketplace(&plan, &mut failed_read).unwrap_err().id(),
        ErrorId::EffectFailed
    );
    assert_eq!(failed_read.bytes, Some(existing_marketplace()));
}

#[test]
fn marketplace_unknown_duplicate_and_compare_exchange_races_fail_closed() {
    let prior = existing_marketplace();
    let mut unknown: Value = serde_json::from_slice(&prior).unwrap();
    unknown["plugins"][0]["surprise"] = json!(true);
    assert!(
        plan_codex_marketplace(
            Some(&serde_json::to_vec(&unknown).unwrap()),
            Some(&digest(&serde_json::to_vec(&unknown).unwrap())),
            "local-harness-plugins",
            "Local Harness Plugins",
            CodexPlugin::harness_ultragoal(),
            package("0.0.12", B, C),
        )
        .is_err()
    );
    let mut duplicate: Value = serde_json::from_slice(&prior).unwrap();
    let duplicate_row = duplicate["plugins"][0].clone();
    duplicate["plugins"]
        .as_array_mut()
        .unwrap()
        .push(duplicate_row);
    let duplicate = serde_json::to_vec(&duplicate).unwrap();
    assert_eq!(
        plan_codex_marketplace(
            Some(&duplicate),
            Some(&digest(&duplicate)),
            "local-harness-plugins",
            "Local Harness Plugins",
            CodexPlugin::harness_ultragoal(),
            package("0.0.12", B, C)
        )
        .unwrap_err()
        .id(),
        ErrorId::InstallConflict
    );
    let plan = plan_codex_marketplace(
        Some(&prior),
        Some(&digest(&prior)),
        "local-harness-plugins",
        "Local Harness Plugins",
        CodexPlugin::harness_ultragoal(),
        package("0.0.12", B, C),
    )
    .unwrap();
    let concurrent = b"concurrent marketplace bytes".to_vec();
    let mut sink = MarketplaceSink {
        bytes: Some(prior),
        race: Some(concurrent.clone()),
        ..MarketplaceSink::default()
    };
    assert_eq!(
        apply_marketplace(&plan, &mut sink).unwrap_err().id(),
        ErrorId::InstallConflict
    );
    assert_eq!(sink.bytes, Some(concurrent));
}
