use super::component_paths::{allowed_children, names_for};
use super::creation::{ProvisionStep, prepare, publish_stage, sync_created_stage};
use super::directory_entries::names;
use super::observation::{open_at, open_root, stat_at, validate_directory};
use super::rollback::OutputClaim;
use super::*;

pub(in crate::routine_work::runtime_adapter::production) struct OutputProvisioning {
    journal: OutputProvisionJournal,
    opened: BTreeMap<String, File>,
    allowed: BTreeMap<String, BTreeSet<String>>,
    index: usize,
    phase: Phase,
    created: Vec<OutputClaim>,
}

enum Phase {
    Inspect,
    SyncCreated(OutputDirectoryIdentity),
    Publish(OutputDirectoryIdentity),
    AwaitStaged(OutputDirectoryIdentity),
    AwaitPublished(OutputDirectoryIdentity),
    Complete,
}

pub(in crate::routine_work::runtime_adapter::production) enum OutputStep {
    Record(OutputTransition),
    Complete(ApplyOutcome),
}

pub(super) fn begin(
    journal: &OutputProvisionJournal,
    root: &Path,
) -> Result<OutputProvisioning, RoutineError> {
    let root = open_root(root)?;
    if identity(
        &root
            .metadata()
            .map_err(|_| error("routine-production-output-root-stat-failed"))?,
    ) != journal.root
    {
        return Err(error("routine-production-output-root-changed"));
    }
    let mut opened = BTreeMap::new();
    opened.insert(String::new(), root);
    Ok(OutputProvisioning {
        journal: journal.clone(),
        opened,
        allowed: allowed_children(journal),
        index: 0,
        phase: Phase::Inspect,
        created: Vec::new(),
    })
}

impl OutputProvisioning {
    pub(in crate::routine_work::runtime_adapter::production) fn next(
        &mut self,
    ) -> Result<OutputStep, RoutineError> {
        loop {
            let component = match self.journal.components.get(self.index) {
                Some(component) => component.clone(),
                None => {
                    self.phase = Phase::Complete;
                    return Ok(OutputStep::Complete(ApplyOutcome::Applied));
                }
            };
            let (parent_name, child_name) = names_for(&component.relative_path);
            let parent = self
                .opened
                .get(parent_name)
                .ok_or_else(|| error("routine-production-output-parent-unbound"))?;
            match self.phase {
                Phase::Inspect => {
                    if let Some(expected) = component.preexisting {
                        if stat_at(parent, child_name)? != Some(expected) {
                            return Err(error("routine-production-output-prestate-changed"));
                        }
                        self.bind_ready(&component, expected)?;
                        continue;
                    }
                    match prepare(&component, parent, child_name, self.journal.root.device)? {
                        ProvisionStep::Ready(identity) => self.bind_ready(&component, identity)?,
                        ProvisionStep::StageCreated(identity) => {
                            let name = component
                                .creation_nonce
                                .as_deref()
                                .map(|nonce| format!(".routine-output-{nonce}"))
                                .ok_or_else(|| {
                                    error("routine-production-output-creation-intent-missing")
                                })?;
                            self.created.push(OutputClaim {
                                parent: parent_name.to_owned(),
                                name,
                                identity,
                            });
                            self.phase = Phase::SyncCreated(identity);
                        }
                        ProvisionStep::StagePresent(identity) => {
                            self.phase = Phase::Publish(identity);
                        }
                        ProvisionStep::Published(identity) => {
                            self.phase = Phase::AwaitPublished(identity);
                            return Ok(OutputStep::Record(OutputTransition::Published {
                                relative_path: component.relative_path,
                                identity,
                            }));
                        }
                        ProvisionStep::UnrecordedStage(ambiguity) => {
                            self.phase = Phase::Complete;
                            return Ok(OutputStep::Complete(ApplyOutcome::UnrecordedStage(
                                ambiguity,
                            )));
                        }
                    }
                }
                Phase::SyncCreated(identity) => {
                    let name = self
                        .created
                        .last()
                        .filter(|claim| claim.identity == identity)
                        .map(|claim| claim.name.as_str())
                        .ok_or_else(|| error("routine-production-output-custody-missing"))?;
                    sync_created_stage(parent, name, identity, self.journal.root.device)?;
                    self.phase = Phase::AwaitStaged(identity);
                    return Ok(OutputStep::Record(OutputTransition::Staged {
                        relative_path: component.relative_path,
                        identity,
                    }));
                }
                Phase::Publish(identity) => {
                    let published = publish_stage(
                        parent,
                        &component,
                        child_name,
                        identity,
                        self.journal.root.device,
                    );
                    if stat_at(parent, child_name)? == Some(identity) {
                        self.rename_claim(identity, child_name);
                    }
                    published?;
                    self.phase = Phase::AwaitPublished(identity);
                    return Ok(OutputStep::Record(OutputTransition::Published {
                        relative_path: component.relative_path,
                        identity,
                    }));
                }
                Phase::AwaitStaged(_) | Phase::AwaitPublished(_) => {
                    return Err(error("routine-production-output-transition-unrecorded"));
                }
                Phase::Complete => {
                    return Err(error("routine-production-output-provisioning-complete"));
                }
            }
        }
    }

    pub(in crate::routine_work::runtime_adapter::production) fn abort(
        self,
    ) -> Result<(), RoutineError> {
        super::rollback::rollback(&self.created, &self.opened)
    }

    pub(in crate::routine_work::runtime_adapter::production) fn accept(self) {}

    pub(in crate::routine_work::runtime_adapter::production) fn recorded(
        &mut self,
        transition: OutputTransition,
    ) -> Result<(), RoutineError> {
        let component = self
            .journal
            .components
            .get(self.index)
            .ok_or_else(|| error("routine-production-output-component-unbound"))?;
        match (&self.phase, transition) {
            (
                Phase::AwaitStaged(expected),
                OutputTransition::Staged {
                    relative_path,
                    identity,
                },
            ) if component.relative_path == relative_path && *expected == identity => {
                self.phase = Phase::Publish(identity);
                Ok(())
            }
            (
                Phase::AwaitPublished(expected),
                OutputTransition::Published {
                    relative_path,
                    identity,
                },
            ) if component.relative_path == relative_path && *expected == identity => {
                let component = component.clone();
                self.bind_ready(&component, identity)
            }
            _ => Err(error("routine-production-output-transition-mismatch")),
        }
    }

    fn bind_ready(
        &mut self,
        component: &OutputComponentJournal,
        expected: OutputDirectoryIdentity,
    ) -> Result<(), RoutineError> {
        validate_directory(expected, self.journal.root.device)?;
        let (parent_name, child_name) = names_for(&component.relative_path);
        let parent = self
            .opened
            .get(parent_name)
            .ok_or_else(|| error("routine-production-output-parent-unbound"))?;
        let child = open_at(parent, child_name)?;
        if identity(
            &child
                .metadata()
                .map_err(|_| error("routine-production-output-stat-failed"))?,
        ) != expected
            || stat_at(parent, child_name)? != Some(expected)
        {
            return Err(error("routine-production-output-open-raced"));
        }
        if component.preexisting.is_none() {
            let expected_names = self
                .allowed
                .get(&component.relative_path)
                .cloned()
                .unwrap_or_default();
            if !names(&child)?.is_subset(&expected_names) {
                return Err(error("routine-production-output-owned-content-ambiguous"));
            }
        }
        self.opened.insert(component.relative_path.clone(), child);
        self.index += 1;
        self.phase = Phase::Inspect;
        Ok(())
    }

    fn rename_claim(&mut self, identity: OutputDirectoryIdentity, name: &str) {
        if let Some(claim) = self
            .created
            .iter_mut()
            .find(|claim| claim.identity == identity)
        {
            claim.name = name.to_owned();
        }
    }
}
