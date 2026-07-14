use super::GovernedSource;
use super::scope::SourceClass;
use std::collections::BTreeSet;

const REQUIRED: &[LiveRootDocument] = &[
    LiveRootDocument::Readme,
    LiveRootDocument::Architecture,
    LiveRootDocument::Plans,
    LiveRootDocument::Security,
    LiveRootDocument::Design,
    LiveRootDocument::Frontend,
    LiveRootDocument::Reliability,
    LiveRootDocument::ProductSense,
    LiveRootDocument::ProductFitness,
    LiveRootDocument::QualityScore,
    LiveRootDocument::ProductSuccessContract,
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum LiveRootDocument {
    Readme,
    Architecture,
    Plans,
    Security,
    Design,
    Frontend,
    Reliability,
    ProductSense,
    ProductFitness,
    QualityScore,
    ProductSuccessContract,
}

impl LiveRootDocument {
    fn from_relative(relative: &str) -> Option<Self> {
        match relative {
            "README.md" => Some(Self::Readme),
            "ARCHITECTURE.md" => Some(Self::Architecture),
            "PLANS.md" => Some(Self::Plans),
            "SECURITY.md" => Some(Self::Security),
            "DESIGN.md" => Some(Self::Design),
            "FRONTEND.md" => Some(Self::Frontend),
            "RELIABILITY.md" => Some(Self::Reliability),
            "PRODUCT_SENSE.md" => Some(Self::ProductSense),
            "PRODUCT_FITNESS.md" => Some(Self::ProductFitness),
            "QUALITY_SCORE.md" => Some(Self::QualityScore),
            "PRODUCT_SUCCESS_CONTRACT.md" => Some(Self::ProductSuccessContract),
            _ => None,
        }
    }

    fn relative(self) -> &'static str {
        match self {
            Self::Readme => "README.md",
            Self::Architecture => "ARCHITECTURE.md",
            Self::Plans => "PLANS.md",
            Self::Security => "SECURITY.md",
            Self::Design => "DESIGN.md",
            Self::Frontend => "FRONTEND.md",
            Self::Reliability => "RELIABILITY.md",
            Self::ProductSense => "PRODUCT_SENSE.md",
            Self::ProductFitness => "PRODUCT_FITNESS.md",
            Self::QualityScore => "QUALITY_SCORE.md",
            Self::ProductSuccessContract => "PRODUCT_SUCCESS_CONTRACT.md",
        }
    }
}

pub(super) fn failures(sources: &[GovernedSource]) -> Vec<String> {
    let mut failures = Vec::new();
    let mut observed = BTreeSet::new();
    for source in sources
        .iter()
        .filter(|source| source.class == SourceClass::LiveRootDocument)
    {
        let Some(identity) = LiveRootDocument::from_relative(&source.relative) else {
            failures.push(format!(
                "live_root_document_identity_unknown:{}",
                source.relative
            ));
            continue;
        };
        observed.insert(identity);
        let Ok(text) = std::str::from_utf8(&source.bytes) else {
            failures.push(format!("live_root_document_non_utf8:{}", source.relative));
            continue;
        };
        let lower = text.to_ascii_lowercase();
        for (id, present) in [
            ("describe_what", lower.contains("describe what")),
            ("replace_these_with", lower.contains("replace these with")),
            (
                "name_the_gate",
                lower.contains("name the ") && lower.contains(" gate"),
            ),
        ] {
            if present {
                failures.push(format!(
                    "live_root_document_unresolved_template:{}:{id}",
                    source.relative
                ));
            }
        }
    }
    for required in REQUIRED {
        if !observed.contains(required) {
            failures.push(format!(
                "live_root_document_missing:{}",
                required.relative()
            ));
        }
    }
    failures.sort();
    failures.dedup();
    failures
}
