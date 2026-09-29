use super::super::*;
use sha2::Digest;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

pub(super) fn root(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "hul-evaluation-custody-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

pub(super) fn execution_binding(candidate: char, session: char) -> EvaluationExecutionBinding {
    EvaluationExecutionBinding::new(EvaluationExecutionBindingRequest {
        live_context_id: sha('a'),
        candidate_id: sha(candidate),
        spec_sha256: sha('c'),
        task_set_sha256: sha('d'),
        execution_session_id: sha(session),
        execution_material_set_sha256: sha('e'),
        artifact_root_sha256: sha('f'),
    })
    .unwrap()
}

fn terminal(root: &Path, key: [u8; 32], candidate: char, session: char, run: char) {
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(root, key, execution_binding(candidate, session))
            .unwrap();
    ledger.reserve().unwrap();
    ledger.publish_result(sha(run), sha('9')).unwrap();
    ledger.complete().unwrap();
}

fn promotion_binding(baseline: &Path, candidate: &Path) -> PromotionLedgerBinding {
    let baseline =
        FileEvaluationExecutionLedger::open(baseline, [1; 32], execution_binding('b', '1'))
            .unwrap();
    let candidate =
        FileEvaluationExecutionLedger::open(candidate, [2; 32], execution_binding('c', '2'))
            .unwrap();
    PromotionLedgerBinding::from_terminal_proofs(
        "promotion-authority",
        "independent-reviewer",
        sha('3'),
        baseline.terminal_proof().unwrap(),
        candidate.terminal_proof().unwrap(),
    )
    .unwrap()
}

fn barrier() -> Result<(), ()> {
    let root = PathBuf::from(std::env::var_os("HUL_EVALUATION_CUSTODY_BARRIER").ok_or(())?);
    let id = std::env::var("HUL_EVALUATION_CUSTODY_PARTICIPANT").map_err(|_| ())?;
    fs::write(root.join(format!("ready-{id}")), b"ready").map_err(|_| ())?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while !root.join("release").is_file() {
        if Instant::now() >= deadline {
            return Err(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}

fn release(root: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !(root.join("ready-0").is_file() && root.join("ready-1").is_file()) {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for custody participants"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    fs::write(root.join("release"), b"release").unwrap();
}

#[test]
fn execution_custody_worker() {
    let Some(root) = std::env::var_os("HUL_EVALUATION_EXECUTION_CUSTODY_ROOT") else {
        return;
    };
    if barrier().is_err() {
        std::process::exit(90);
    }
    let mut ledger =
        FileEvaluationExecutionLedger::open(root, [4; 32], execution_binding('b', '1')).unwrap();
    match ledger.reserve_outcome(&sha('7')).unwrap() {
        ExecutionReservationOutcome::Acquired => std::process::exit(80),
        ExecutionReservationOutcome::Lost { .. }
        | ExecutionReservationOutcome::AlreadyReserved
        | ExecutionReservationOutcome::AlreadyPublished { .. }
        | ExecutionReservationOutcome::Interrupted { .. }
        | ExecutionReservationOutcome::RecoveryRequired { .. }
        | ExecutionReservationOutcome::Terminal { .. } => std::process::exit(81),
    }
}

#[test]
fn execution_reservation_has_one_two_process_winner() {
    let ledger_root = root("execution-race");
    FileEvaluationExecutionLedger::initialize(&ledger_root, [4; 32], execution_binding('b', '1'))
        .unwrap();
    let barrier_root = root("execution-barrier");
    let exe = std::env::current_exe().unwrap();
    let mut children = (0..2)
        .map(|id| {
            Command::new(&exe)
                .args([
                    "--exact",
                    "evaluation::tests::custody::execution_custody_worker",
                    "--nocapture",
                ])
                .env("HUL_EVALUATION_EXECUTION_CUSTODY_ROOT", &ledger_root)
                .env("HUL_EVALUATION_CUSTODY_BARRIER", &barrier_root)
                .env("HUL_EVALUATION_CUSTODY_PARTICIPANT", id.to_string())
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    release(&barrier_root);
    let mut codes = children
        .iter_mut()
        .map(|child| child.wait().unwrap().code().unwrap())
        .collect::<Vec<_>>();
    codes.sort_unstable();
    assert_eq!(codes, vec![80, 81]);
    fs::remove_dir_all(ledger_root).unwrap();
    fs::remove_dir_all(barrier_root).unwrap();
}

#[test]
fn promotion_custody_worker() {
    let Some(review_root) = std::env::var_os("HUL_EVALUATION_PROMOTION_CUSTODY_ROOT") else {
        return;
    };
    if barrier().is_err() {
        std::process::exit(90);
    }
    let baseline = PathBuf::from(std::env::var_os("HUL_EVALUATION_BASELINE_ROOT").unwrap());
    let candidate = PathBuf::from(std::env::var_os("HUL_EVALUATION_CANDIDATE_ROOT").unwrap());
    let mut ledger = FilePromotionReviewLedger::open(
        review_root,
        [5; 32],
        promotion_binding(&baseline, &candidate),
    )
    .unwrap();
    let binding = sha('6');
    let attestation = std::env::var("HUL_EVALUATION_ATTESTATION").unwrap();
    let review = format!(
        "sha256:{:x}",
        sha2::Sha256::digest(format!("promotion-review|{binding}|{attestation}").as_bytes())
    );
    if matches!(
        ledger
            .consume_attestation(&binding, "independent-reviewer", &review, &attestation)
            .unwrap(),
        super::super::promotion_ledger::PromotionConsumptionOutcome::Consumed
    ) {
        std::process::exit(83);
    }
    assert!(matches!(
        ledger.inspect().unwrap(),
        PromotionLedgerState::Consumed { .. }
    ));
    std::process::exit(84)
}

#[test]
fn promotion_consumption_has_one_two_process_winner() {
    let baseline = root("promotion-baseline");
    let candidate = root("promotion-candidate");
    let review = root("promotion-review");
    terminal(&baseline, [1; 32], 'b', '1', '4');
    terminal(&candidate, [2; 32], 'c', '2', '5');
    let mut ledger = FilePromotionReviewLedger::initialize(
        &review,
        [5; 32],
        promotion_binding(&baseline, &candidate),
    )
    .unwrap();
    let attestation = ledger.issue_bound_attestation(&sha('6')).unwrap();
    drop(ledger);
    let barrier_root = root("promotion-barrier");
    let exe = std::env::current_exe().unwrap();
    let mut children = (0..2)
        .map(|id| {
            Command::new(&exe)
                .args([
                    "--exact",
                    "evaluation::tests::custody::promotion_custody_worker",
                    "--nocapture",
                ])
                .env("HUL_EVALUATION_PROMOTION_CUSTODY_ROOT", &review)
                .env("HUL_EVALUATION_BASELINE_ROOT", &baseline)
                .env("HUL_EVALUATION_CANDIDATE_ROOT", &candidate)
                .env("HUL_EVALUATION_ATTESTATION", &attestation)
                .env("HUL_EVALUATION_CUSTODY_BARRIER", &barrier_root)
                .env("HUL_EVALUATION_CUSTODY_PARTICIPANT", id.to_string())
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    release(&barrier_root);
    let mut codes = children
        .iter_mut()
        .map(|child| child.wait().unwrap().code().unwrap())
        .collect::<Vec<_>>();
    codes.sort_unstable();
    assert_eq!(codes, vec![83, 84]);
    for path in [baseline, candidate, review, barrier_root] {
        fs::remove_dir_all(path).unwrap();
    }
}
