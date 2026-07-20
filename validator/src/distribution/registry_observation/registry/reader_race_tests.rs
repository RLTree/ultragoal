use super::*;
use crate::distribution::{PackageIdentity, SourceIdentity};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct MutatingRegistry {
    rows: Vec<Vec<u8>>,
    reads: usize,
}

impl RegistryReader for MutatingRegistry {
    fn read_registry(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        let value = self
            .rows
            .get(self.reads)
            .cloned()
            .or_else(|| self.rows.last().cloned());
        self.reads += 1;
        Ok(value)
    }
}

struct Fixture {
    root: PathBuf,
    binding: JourneyBinding,
    host: HostCapabilityDeclaration,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "hul-registry-reader-race-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let home = root.join("home");
        let project = root.join("project");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&project).unwrap();
        let source = SourceIdentity::new(
            digest('1'),
            digest('2'),
            "harness-ultragoal".to_owned(),
            "0.0.11".to_owned(),
            digest('3'),
            digest('4'),
        )
        .unwrap();
        let package = PackageIdentity::new(source, digest('5'), digest('6')).unwrap();
        let host = HostCapabilityDeclaration::isolated(&home, &project, "registry-reader-v1", None)
            .unwrap();
        let binding = JourneyBinding::new(package, &host, "local-harness-plugins").unwrap();
        Self {
            root,
            binding,
            host,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

#[test]
fn registry_reader_race_refuses_changed_rows() {
    let fixture = Fixture::new();
    let mut registry = MutatingRegistry {
        rows: vec![
            registry_document(&fixture.binding, true, true).unwrap(),
            registry_document(&fixture.binding, true, false).unwrap(),
        ],
        reads: 0,
    };

    assert_eq!(
        observe_registry_reader(&mut registry, &fixture.binding, &fixture.host)
            .unwrap_err()
            .id(),
        DistributionErrorId::ObjectChanged
    );
}
