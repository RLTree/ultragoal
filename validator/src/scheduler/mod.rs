use serde_json::{Value, json};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[cfg(test)]
mod tests;

const MAX_EXPLICIT_JOBS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaskClass {
    PureReadParallel,
    IsolatedTempWriteParallel,
    ExternalLiveBoundedParallel,
    SharedAuthorityWriteSerial,
    DestructiveOrMutatingSerial,
}

impl TaskClass {
    const ALL: [Self; 5] = [
        Self::PureReadParallel,
        Self::IsolatedTempWriteParallel,
        Self::ExternalLiveBoundedParallel,
        Self::SharedAuthorityWriteSerial,
        Self::DestructiveOrMutatingSerial,
    ];

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::PureReadParallel => "pure_read_parallel",
            Self::IsolatedTempWriteParallel => "isolated_temp_write_parallel",
            Self::ExternalLiveBoundedParallel => "external_live_bounded_parallel",
            Self::SharedAuthorityWriteSerial => "shared_authority_write_serial",
            Self::DestructiveOrMutatingSerial => "destructive_or_mutating_serial",
        }
    }

    fn allows_parallel(self) -> bool {
        matches!(
            self,
            Self::PureReadParallel
                | Self::IsolatedTempWriteParallel
                | Self::ExternalLiveBoundedParallel
        )
    }

    fn is_known_id(id: &str) -> bool {
        Self::ALL.iter().any(|class| class.id() == id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SchedulerConfig {
    jobs: usize,
}

impl SchedulerConfig {
    pub(crate) fn from_jobs(jobs: Option<usize>) -> Result<Self, String> {
        let jobs = match jobs {
            Some(0) => return Err("scheduler jobs must be at least 1".to_string()),
            Some(value) if value > MAX_EXPLICIT_JOBS => {
                return Err(format!(
                    "scheduler jobs exceeds bounded maximum {MAX_EXPLICIT_JOBS}"
                ));
            }
            Some(value) => value,
            None => default_worker_count(),
        };
        Ok(Self { jobs })
    }

    pub(crate) fn jobs(self) -> usize {
        self.jobs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Metrics {
    pub(crate) task_class: &'static str,
    pub(crate) worker_count: usize,
    pub(crate) task_count: usize,
    pub(crate) queue_depth: usize,
    pub(crate) wall_ms: u128,
    pub(crate) cache_mode: &'static str,
    pub(crate) deterministic_ordering: bool,
    pub(crate) shared_validation_artifact_writes_allowed: bool,
}

impl Metrics {
    pub(crate) fn to_value(&self, candidate_digest: &str, claim_impact: &str) -> Value {
        debug_assert!(TaskClass::is_known_id(self.task_class));
        json!({
            "task_class": self.task_class,
            "worker_count": self.worker_count,
            "task_count": self.task_count,
            "queue_depth": self.queue_depth,
            "wall_ms": self.wall_ms,
            "cache_mode": self.cache_mode,
            "candidate_digest": candidate_digest,
            "deterministic_ordering": self.deterministic_ordering,
            "shared_validation_artifact_writes_allowed": self.shared_validation_artifact_writes_allowed,
            "claim_impact": claim_impact
        })
    }
}

pub(crate) struct Scheduled<T> {
    pub(crate) values: Vec<T>,
    pub(crate) metrics: Metrics,
}

type Task<T> = Box<dyn FnOnce() -> T + Send + 'static>;

pub(crate) fn run_ordered<T: Send + 'static>(
    config: SchedulerConfig,
    task_class: TaskClass,
    tasks: Vec<Task<T>>,
) -> Scheduled<T> {
    let start = Instant::now();
    let task_count = tasks.len();
    let queue_depth = task_count;
    let worker_count = worker_count(config, task_class, task_count);
    let mut indexed_tasks = tasks.into_iter().enumerate().collect::<VecDeque<_>>();

    let results = if worker_count == 1 {
        let mut out = Vec::with_capacity(task_count);
        while let Some((index, task)) = indexed_tasks.pop_front() {
            out.push((index, task()));
        }
        out
    } else {
        run_parallel(worker_count, indexed_tasks)
    };

    let mut results = results;
    results.sort_by_key(|(index, _)| *index);
    let values = results.into_iter().map(|(_, value)| value).collect();
    Scheduled {
        values,
        metrics: Metrics {
            task_class: task_class.id(),
            worker_count,
            task_count,
            queue_depth,
            wall_ms: start.elapsed().as_millis(),
            cache_mode: "declared_local",
            deterministic_ordering: true,
            shared_validation_artifact_writes_allowed: false,
        },
    }
}

fn run_parallel<T: Send + 'static>(
    worker_count: usize,
    tasks: VecDeque<(usize, Task<T>)>,
) -> Vec<(usize, T)> {
    let queue = Arc::new(Mutex::new(tasks));
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let queue = Arc::clone(&queue);
        let results = Arc::clone(&results);
        handles.push(std::thread::spawn(move || {
            loop {
                let next = queue.lock().expect("scheduler queue").pop_front();
                let Some((index, task)) = next else {
                    break;
                };
                let value = task();
                results
                    .lock()
                    .expect("scheduler results")
                    .push((index, value));
            }
        }));
    }
    for handle in handles {
        handle.join().expect("scheduler worker");
    }
    let results = match Arc::try_unwrap(results) {
        Ok(results) => results,
        Err(_) => panic!("scheduler results shared"),
    };
    results.into_inner().expect("scheduler results")
}

fn worker_count(config: SchedulerConfig, task_class: TaskClass, task_count: usize) -> usize {
    if task_count == 0 || !task_class.allows_parallel() {
        return usize::from(task_count > 0);
    }
    config.jobs().min(task_count).max(1)
}

fn default_worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|count| count.get().saturating_sub(1).max(1))
        .unwrap_or(1)
}
