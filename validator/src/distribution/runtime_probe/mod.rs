use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::host_capability::SUPPORTED_RUNTIME_PROGRAM;
use crate::distribution::host_capability::{
    HostCapabilityDeclaration, HostCapabilityState, JourneyBinding,
};
use crate::distribution::install::{CurrentInstallAuthority, InstallSnapshot};
use crate::distribution::json;
use crate::distribution::model::Capability;
use crate::distribution::model::{IdentitySurface, SurfaceIdentity};
use crate::distribution::observations::RuntimeObservation;
use crate::distribution::package::{PackageRole, PackageSnapshot};
use crate::distribution::reader::sha256;
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

include!("pinned_executable.rs");

include!("execution_copy.rs");

include!("help_envelope.rs");

include!("output_limit.rs");

include!("runtime_execution.rs");

include!("unique_marker.rs");

const PACKAGE_RUNTIME_ENTRY: &str = "runtime/ultragoal";
