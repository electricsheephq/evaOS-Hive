use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::process::ExitStatus;
use std::time::{Duration, Instant};

use crate::managed_agents::readiness::cli_probe;
use crate::managed_agents::{AcpAvailabilityStatus, AuthStatus};

use super::RuntimeAuthProbe;

pub(super) fn run_probe(
    binary_path: &Path,
    probe_args: &[&str],
    timeout: Duration,
) -> Option<(ExitStatus, Vec<u8>)> {
    let augmented_path = cli_probe::augmented_path();
    let mut stderr = tempfile::tempfile().ok()?;
    let mut command = std::process::Command::new(binary_path);
    command.args(&probe_args[1..]);
    if let Some(ref path) = augmented_path {
        command.env("PATH", path);
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(stderr.try_clone().ok()?);

    let mut child = command.spawn().ok()?;
    let deadline = Instant::now() + timeout;
    let exit_status = loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(100).min(remaining)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };

    stderr.seek(SeekFrom::Start(0)).ok()?;
    let mut stderr_bytes = Vec::with_capacity(256);
    let _ = (&mut stderr as &mut dyn Read)
        .take(4096)
        .read_to_end(&mut stderr_bytes);
    Some((exit_status, stderr_bytes))
}

/// Run a CLI authentication probe with a 10-second process-level timeout.
pub(super) fn probe_auth_status(binary_path: &Path, probe_args: &[&str]) -> AuthStatus {
    use crate::managed_agents::readiness::cli_probe;

    let Some((exit_status, stderr_bytes)) =
        run_probe(binary_path, probe_args, Duration::from_secs(10))
    else {
        return AuthStatus::Unknown;
    };
    match cli_probe::classify_probe_output(&stderr_bytes, exit_status.success()) {
        cli_probe::ProbeOutcome::LoggedIn => AuthStatus::LoggedIn,
        cli_probe::ProbeOutcome::LoggedOut => AuthStatus::LoggedOut,
        cli_probe::ProbeOutcome::ConfigInvalid { stderr_excerpt } => AuthStatus::ConfigInvalid {
            diagnostic: stderr_excerpt,
        },
    }
}

pub(super) fn availability_after_readiness_probe(
    availability: AcpAvailabilityStatus,
    binary_path: Option<&Path>,
    probe_args: &[&str],
) -> AcpAvailabilityStatus {
    if availability != AcpAvailabilityStatus::Available {
        return availability;
    }
    match binary_path.and_then(|path| run_probe(path, probe_args, Duration::from_secs(10))) {
        Some((status, _)) if status.success() => AcpAvailabilityStatus::Available,
        _ => AcpAvailabilityStatus::DependencyMissing,
    }
}

pub(super) fn auth_status_without_probe(
    availability: &AcpAvailabilityStatus,
    auth_probe: &RuntimeAuthProbe,
) -> AuthStatus {
    if *availability != AcpAvailabilityStatus::Available {
        return AuthStatus::Unknown;
    }
    match auth_probe {
        RuntimeAuthProbe::AcpHandshake => AuthStatus::CheckedOnLaunch,
        RuntimeAuthProbe::NotApplicable => AuthStatus::NotApplicable,
        RuntimeAuthProbe::Cli(_) => AuthStatus::Unknown,
    }
}
