use super::creation::{ProvisionStep, prepare, publish_stage};
use super::directory_entries::names;
use super::observation::{open_at, open_root, stat_at, validate_directory};
use super::*;

pub(in crate::routine_work::runtime_adapter::production) struct OutputProvisioning {
    journal: OutputProvisionJournal,
    opened: BTreeMap<String, File>,
    allowed: BTreeMap<String, BTreeSet<String>>,
    index: usize,
    phase: Phase,
}

enum Phase {
    Inspect,
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
                            self.phase = Phase::AwaitStaged(identity);
                            return Ok(OutputStep::Record(OutputTransition::Staged {
                                relative_path: component.relative_path,
                                identity,
                            }));
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
                Phase::Publish(identity) => {
                    publish_stage(
                        parent,
                        &component,
                        child_name,
                        identity,
                        self.journal.root.device,
                    )?;
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
}

fn names_for(relative: &str) -> (&str, &str) {
    relative
        .rsplit_once('/')
        .map_or(("", relative), |(parent, child)| (parent, child))
}

fn allowed_children(journal: &OutputProvisionJournal) -> BTreeMap<String, BTreeSet<String>> {
    let mut allowed = BTreeMap::<String, BTreeSet<String>>::new();
    for component in &journal.components {
        if let Some((parent, child)) = component.relative_path.rsplit_once('/') {
            let children = allowed.entry(parent.to_owned()).or_default();
            children.insert(child.to_owned());
            if component.preexisting.is_none()
                && let Some(nonce) = component.creation_nonce.as_deref()
            {
                children.insert(format!(".routine-output-{nonce}"));
            }
        }
    }
    allowed
}
