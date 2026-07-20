use super::model::{LifecycleError, LifecyclePlan, LifecycleState};
use super::plan::validate_plan;

pub fn verify(actual: &LifecycleState, plan: &LifecyclePlan) -> Result<(), LifecycleError> {
    validate_plan(plan)?;
    actual.validate()?;
    if actual != &plan.expected_after {
        return Err(LifecycleError::VerificationFailed);
    }
    Ok(())
}
