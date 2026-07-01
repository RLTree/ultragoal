use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct ProductCommand {
    pub(crate) operation: ProductOperation,
    pub(crate) receipt_dir: Option<PathBuf>,
    pub(crate) observability_receipt: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProductOperation {
    ProductProveCohesion,
    ProductProveFitness,
    ProductProveJourney,
    FitRepoProve,
}

impl ProductOperation {
    pub(crate) fn command(self) -> &'static str {
        match self {
            Self::ProductProveCohesion | Self::ProductProveFitness | Self::ProductProveJourney => {
                "ultragoal product"
            }
            Self::FitRepoProve => "ultragoal fit-repo",
        }
    }

    pub(crate) fn subcommand(self) -> &'static str {
        match self {
            Self::ProductProveCohesion => "prove-cohesion",
            Self::ProductProveFitness => "prove-fitness",
            Self::ProductProveJourney => "prove-journey",
            Self::FitRepoProve => "prove",
        }
    }

    pub(crate) fn telemetry_operation(self) -> &'static str {
        match self {
            Self::ProductProveCohesion => "product.prove-cohesion",
            Self::ProductProveFitness => "product.prove-fitness",
            Self::ProductProveJourney => "product.prove-journey",
            Self::FitRepoProve => "fit-repo.prove",
        }
    }

    pub(crate) fn check_id(self) -> &'static str {
        match self {
            Self::ProductProveCohesion => "product-prove-cohesion-observability-binding",
            Self::ProductProveFitness => "product-prove-fitness-observability-binding",
            Self::ProductProveJourney => "product-prove-journey-observability-binding",
            Self::FitRepoProve => "fit-repo-prove-observability-binding",
        }
    }

    pub(crate) fn claim_id(self) -> &'static str {
        match self {
            Self::ProductProveCohesion => "product_cohesion",
            Self::ProductProveFitness => "product_fitness",
            Self::ProductProveJourney => "plugin_product_journey",
            Self::FitRepoProve => "fit_repo",
        }
    }

    pub(crate) fn law_id(self) -> &'static str {
        match self {
            Self::ProductProveCohesion => "product-cohesion-gate",
            Self::ProductProveJourney => {
                "plugin-flow-graph-package-dependency-closure-plugin-product-journey"
            }
            Self::ProductProveFitness | Self::FitRepoProve => "product-fitness-gate",
        }
    }

    pub(crate) fn artifact_path(self) -> &'static str {
        match self {
            Self::ProductProveCohesion => {
                "docs/product-cohesion.md,validation_artifacts/product-cohesion/journey-receipt.json"
            }
            Self::ProductProveFitness | Self::ProductProveJourney | Self::FitRepoProve => {
                "validation_artifacts/harness"
            }
        }
    }

    pub(crate) fn report_failure_class(self) -> &'static str {
        match self {
            Self::ProductProveCohesion => "product_cohesion_failure",
            Self::ProductProveJourney => "plugin_product_journey_failure",
            Self::ProductProveFitness | Self::FitRepoProve => "product_receipt_failure",
        }
    }

    pub(crate) fn next_repair(self) -> &'static str {
        match self {
            Self::ProductProveCohesion => {
                "create docs/product-cohesion.md and validation_artifacts/product-cohesion/journey-receipt.json with valid evidence, then rerun product prove-cohesion"
            }
            Self::ProductProveFitness => {
                "query this run through observe logs/metrics/traces, repair the named product receipt or receipt-dir failure, then rerun product prove-fitness"
            }
            Self::ProductProveJourney => {
                "query this run through observe logs/metrics/traces, repair the plugin product journey receipt or receipt-dir failure, then rerun product prove-journey"
            }
            Self::FitRepoProve => {
                "query this run through observe logs/metrics/traces, repair the named fit-repo receipt or receipt-dir failure, then rerun fit-repo prove"
            }
        }
    }

    pub(crate) fn pass_claim_impact(self) -> &'static str {
        match self {
            Self::ProductProveCohesion => {
                "supports_product_cohesion_source_local_observability_only"
            }
            Self::ProductProveFitness => "supports_product_fitness_source_local_observability_only",
            Self::ProductProveJourney => {
                "supports_plugin_product_journey_source_local_observability_only"
            }
            Self::FitRepoProve => "supports_fit_repo_source_local_observability_only",
        }
    }

    pub(crate) fn fail_claim_impact(self) -> &'static str {
        match self {
            Self::ProductProveCohesion => {
                "product_cohesion_failed_blocks_readiness_release_completion_update_goal"
            }
            Self::ProductProveFitness => {
                "product_fitness_failed_blocks_readiness_release_completion_update_goal"
            }
            Self::ProductProveJourney => {
                "plugin_product_journey_failed_blocks_readiness_release_completion_update_goal"
            }
            Self::FitRepoProve => "fit_repo_failed_blocks_readiness_release_completion_update_goal",
        }
    }

    pub(crate) fn receipt_rel(self) -> &'static str {
        match self {
            Self::ProductProveCohesion => {
                "validation_artifacts/observability/product-prove-cohesion.json"
            }
            Self::ProductProveFitness => {
                "validation_artifacts/observability/product-prove-fitness.json"
            }
            Self::ProductProveJourney => {
                "validation_artifacts/observability/product-prove-journey.json"
            }
            Self::FitRepoProve => "validation_artifacts/observability/fit-repo-prove.json",
        }
    }
}
