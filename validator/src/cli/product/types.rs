use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct ProductCommand {
    pub(crate) operation: ProductOperation,
    pub(crate) receipt_dir: PathBuf,
    pub(crate) observability_receipt: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProductOperation {
    ProductProveFitness,
    FitRepoProve,
}

impl ProductOperation {
    pub(crate) fn command(self) -> &'static str {
        match self {
            Self::ProductProveFitness => "ultragoal product",
            Self::FitRepoProve => "ultragoal fit-repo",
        }
    }

    pub(crate) fn subcommand(self) -> &'static str {
        match self {
            Self::ProductProveFitness => "prove-fitness",
            Self::FitRepoProve => "prove",
        }
    }

    pub(crate) fn telemetry_operation(self) -> &'static str {
        match self {
            Self::ProductProveFitness => "product.prove-fitness",
            Self::FitRepoProve => "fit-repo.prove",
        }
    }

    pub(crate) fn check_id(self) -> &'static str {
        match self {
            Self::ProductProveFitness => "product-prove-fitness-observability-binding",
            Self::FitRepoProve => "fit-repo-prove-observability-binding",
        }
    }

    pub(crate) fn claim_id(self) -> &'static str {
        match self {
            Self::ProductProveFitness => "product_fitness",
            Self::FitRepoProve => "fit_repo",
        }
    }

    pub(crate) fn receipt_rel(self) -> &'static str {
        match self {
            Self::ProductProveFitness => {
                "validation_artifacts/observability/product-prove-fitness.json"
            }
            Self::FitRepoProve => "validation_artifacts/observability/fit-repo-prove.json",
        }
    }
}
