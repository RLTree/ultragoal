use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize)]
struct CoverageReport {
    data: Vec<CoverageReportData>,
}

#[derive(serde::Deserialize)]
struct CoverageReportData {
    totals: CoverageReportTotals,
    #[serde(default)]
    files: Vec<CoverageReportFile>,
}

#[derive(serde::Deserialize)]
struct CoverageReportTotals {
    lines: CoverageReportLines,
}

#[derive(serde::Deserialize)]
struct CoverageReportFile {
    filename: String,
    summary: CoverageReportSummary,
}

#[derive(serde::Deserialize)]
struct CoverageReportSummary {
    lines: CoverageReportLines,
}

#[derive(serde::Deserialize)]
struct CoverageReportLines {
    percent: f64,
}

pub(crate) enum CoverageReportRequest<'a> {
    Validate { path: &'a Path },
    Observe { root: &'a Path, path: &'a Path },
}

pub(crate) enum CoverageReportResponse {
    Validated,
    Observation(CoverageReportObservation),
}

pub(crate) struct CoverageReportObservation {
    pub(crate) line_percent: f64,
    pub(crate) uncovered_files: Vec<CoverageReportUncoveredFile>,
}

pub(crate) struct CoverageReportUncoveredFile {
    pub(crate) path: String,
    pub(crate) line_percent: f64,
}

#[derive(Debug)]
pub(crate) struct CoverageReportError {
    pub(crate) code: &'static str,
    pub(crate) path: PathBuf,
    pub(crate) detail: String,
}

impl fmt::Display for CoverageReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:{}",
            self.code,
            self.path.display(),
            self.detail
        )
    }
}

pub(crate) fn execute(
    request: CoverageReportRequest<'_>,
) -> Result<CoverageReportResponse, CoverageReportError> {
    let path = match request {
        CoverageReportRequest::Validate { path } | CoverageReportRequest::Observe { path, .. } => {
            path
        }
    };
    let bytes =
        fs::read(path).map_err(|error| report_error("coverage_report_read_failed", path, error))?;
    let report = serde_json::from_slice::<CoverageReport>(&bytes)
        .map_err(|error| report_error("coverage_report_not_machine_readable", path, error))?;
    match request {
        CoverageReportRequest::Validate { .. } => Ok(CoverageReportResponse::Validated),
        CoverageReportRequest::Observe { root, .. } => Ok(CoverageReportResponse::Observation(
            observation(root, path, &report)?,
        )),
    }
}

fn observation(
    root: &Path,
    path: &Path,
    report: &CoverageReport,
) -> Result<CoverageReportObservation, CoverageReportError> {
    let data = report.data.first().ok_or_else(|| {
        report_error(
            "coverage_report_total_percent_missing",
            path,
            "missing report data",
        )
    })?;
    let line_percent = data.totals.lines.percent;
    let canonical_root = root
        .canonicalize()
        .map_err(|error| report_error("coverage_report_root_canonicalize_failed", root, error))?;
    let mut uncovered_files = Vec::new();
    for item in &data.files {
        let percent = item.summary.lines.percent;
        if percent >= 100.0 {
            continue;
        }
        uncovered_files.push(CoverageReportUncoveredFile {
            path: report_path_label(&canonical_root, &item.filename),
            line_percent: percent,
        });
    }
    Ok(CoverageReportObservation {
        line_percent,
        uncovered_files,
    })
}

fn report_path_label(root: &Path, filename: &str) -> String {
    let path = PathBuf::from(filename);
    let resolved = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    resolved
        .strip_prefix(root)
        .map(|relative| relative.to_string_lossy().to_string())
        .unwrap_or_else(|_| filename.to_string())
}

fn report_error(code: &'static str, path: &Path, detail: impl fmt::Display) -> CoverageReportError {
    CoverageReportError {
        code,
        path: path.to_path_buf(),
        detail: detail.to_string(),
    }
}
