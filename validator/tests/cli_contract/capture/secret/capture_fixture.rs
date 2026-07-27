use super::capture::{ArtifactDisposition, ArtifactRef, ArtifactResolver, CapturedArtifact};
use super::fixture::RepoFixture;
use crate::context::{BuildRequest, LiveContext};
use sha2::{Digest, Sha256};

pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn bound_context(
    fixture: &RepoFixture,
    sources: &[(&str, &str)],
    capabilities: &[&str],
) -> LiveContext {
    let mut request = BuildRequest::new(fixture.root())
        .expect_repository_root(fixture.root())
        .expect_worktree_root(fixture.root());
    for (name, version) in sources {
        request = request.bind_secret_source(*name, *version);
    }
    for capability in capabilities {
        request = request.probe_tool(*capability);
    }
    LiveContext::build(request).expect("build artifact secret context")
}

pub fn assert_withheld(artifact: &CapturedArtifact, needles: &[&[u8]]) {
    assert_eq!(
        artifact.disposition(),
        ArtifactDisposition::WithheldSecretBearingInvocation
    );
    assert!(artifact.bytes().is_empty());
    assert_eq!(artifact.byte_length(), 0);
    assert_eq!(artifact.sha256(), digest(&[]));
    assert_eq!(artifact.relative_path(), None);

    let reference: ArtifactRef = artifact.artifact_ref();
    assert_eq!(reference.disposition(), artifact.disposition());
    assert_eq!(reference.sha256(), artifact.sha256());
    assert_eq!(reference.byte_length(), 0);
    assert!(artifact.resolve(&reference).unwrap().is_empty());

    let surfaces = [
        format!("{artifact:?}").into_bytes(),
        format!("{reference:?}").into_bytes(),
        artifact.to_canonical_json().unwrap(),
        reference.to_canonical_json().unwrap(),
    ];
    for surface in &surfaces {
        assert_serialized_absent(surface, needles);
    }
}

pub fn assert_public(artifact: &CapturedArtifact, expected: &[u8]) {
    assert_eq!(artifact.disposition(), ArtifactDisposition::Public);
    assert_eq!(artifact.bytes(), expected);
    assert_eq!(artifact.byte_length(), expected.len() as u64);
    assert_eq!(artifact.sha256(), digest(expected));
    let reference: ArtifactRef = artifact.artifact_ref();
    assert_eq!(reference.disposition(), ArtifactDisposition::Public);
    assert_eq!(&*artifact.resolve(&reference).unwrap(), expected);
}

pub fn assert_serialized_absent(surface: &[u8], needles: &[&[u8]]) {
    let text = String::from_utf8_lossy(surface);
    for needle in needles.iter().copied().filter(|needle| !needle.is_empty()) {
        assert!(
            !surface.windows(needle.len()).any(|seen| seen == needle),
            "raw secret-derived bytes were observable"
        );
        let raw_digest = digest(needle);
        let bare_digest = raw_digest
            .strip_prefix("sha256:")
            .expect("test digest prefix");
        assert!(
            !text.contains(&raw_digest),
            "raw secret-derived digest was observable"
        );
        assert!(
            !text.contains(bare_digest),
            "bare secret-derived digest was observable"
        );
        assert!(
            !text.contains(&format!("{needle:?}")),
            "decimal secret-derived bytes were observable in Debug"
        );
        let compact_json = format!(
            "[{}]",
            needle
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
        assert!(
            !text.contains(&compact_json),
            "decimal secret-derived bytes were observable in compact JSON"
        );
        if needle.len() >= 4 {
            let encoded = needle
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            assert!(
                !text.contains(&encoded),
                "hex-encoded secret-derived bytes were observable"
            );
        }
    }
}
