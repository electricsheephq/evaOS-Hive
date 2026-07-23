use super::super::{
    auth_status_without_probe, availability_after_readiness_probe, managed_agent_avatar_url,
    normalize_agent_args, run_probe, runtime_readiness_probe_args, RuntimeAuthProbe,
    HERMES_AVATAR_URL,
};
use crate::managed_agents::{AcpAvailabilityStatus, AuthStatus};
use std::time::{Duration, Instant};

#[test]
fn resolves_supported_hermes_commands() {
    assert_eq!(
        managed_agent_avatar_url("/usr/local/bin/hermes"),
        Some(HERMES_AVATAR_URL.to_string())
    );
    assert_eq!(
        managed_agent_avatar_url(r"C:\Tools\hermes-acp.exe"),
        Some(HERMES_AVATAR_URL.to_string())
    );
}

#[test]
fn normalizes_args_for_each_supported_command() {
    assert_eq!(
        normalize_agent_args("hermes", Vec::new()),
        vec!["acp".to_string()]
    );
    assert_eq!(
        normalize_agent_args("/usr/local/bin/hermes", Vec::new()),
        vec!["acp".to_string()]
    );
    assert_eq!(
        normalize_agent_args("hermes-acp", vec!["acp".into()]),
        Vec::<String>::new()
    );
    assert_eq!(
        runtime_readiness_probe_args("hermes", &["--check"]),
        vec!["hermes", "acp", "--check"]
    );
    assert_eq!(
        runtime_readiness_probe_args("hermes-acp", &["--check"]),
        vec!["hermes-acp", "--check"]
    );
}

#[test]
fn metadata_separates_dependency_readiness_from_authentication() {
    let hermes = super::super::known_acp_runtime_exact("hermes").expect("Hermes metadata");

    assert_eq!(hermes.commands, &["hermes", "hermes-acp"]);
    assert_eq!(hermes.underlying_cli, None);
    assert_eq!(hermes.readiness_probe_suffix, Some(&["--check"][..]));
    assert!(matches!(hermes.auth_probe, RuntimeAuthProbe::AcpHandshake));
    assert_eq!(hermes.skill_dir, Some(".hermes/skills"));
    assert_eq!(hermes.config_file_path, Some("~/.hermes/config.yaml"));
    assert_eq!(hermes.config_file_format, Some("yaml"));
    assert_eq!(hermes.model_env_var, None);
    assert_eq!(hermes.provider_env_var, None);
    assert_eq!(
        auth_status_without_probe(&AcpAvailabilityStatus::Available, &hermes.auth_probe),
        AuthStatus::CheckedOnLaunch
    );
    assert_ne!(
        auth_status_without_probe(&AcpAvailabilityStatus::Available, &hermes.auth_probe),
        AuthStatus::LoggedIn,
        "dependency readiness must never become an authentication claim"
    );
}

#[test]
fn readiness_probe_distinguishes_success_failure_and_unavailable() {
    let executable = std::env::current_exe().expect("current test executable");
    let executable_str = executable.to_string_lossy();

    assert_eq!(
        availability_after_readiness_probe(
            AcpAvailabilityStatus::Available,
            Some(&executable),
            &[executable_str.as_ref(), "--list"],
        ),
        AcpAvailabilityStatus::Available
    );
    assert_eq!(
        availability_after_readiness_probe(
            AcpAvailabilityStatus::Available,
            Some(&executable),
            &[executable_str.as_ref(), "--buzz-invalid-readiness-probe"],
        ),
        AcpAvailabilityStatus::DependencyMissing
    );
    assert_eq!(
        availability_after_readiness_probe(
            AcpAvailabilityStatus::Available,
            None,
            &["missing", "--check"],
        ),
        AcpAvailabilityStatus::DependencyMissing
    );
    assert_eq!(
        availability_after_readiness_probe(
            AcpAvailabilityStatus::NotInstalled,
            Some(&executable),
            &[executable_str.as_ref(), "--list"],
        ),
        AcpAvailabilityStatus::NotInstalled
    );
}

#[test]
fn readiness_probe_timeout_is_bounded() {
    let executable = std::env::current_exe().expect("current test executable");
    let executable_str = executable.to_string_lossy();
    let started = Instant::now();
    let result = run_probe(
        &executable,
        &[
            executable_str.as_ref(),
            "--exact",
            "managed_agents::discovery::tests::hermes::timeout_probe_fixture",
            "--ignored",
        ],
        Duration::from_millis(100),
    );

    assert!(result.is_none());
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
#[ignore = "subprocess fixture for readiness_probe_timeout_is_bounded"]
fn timeout_probe_fixture() {
    std::thread::sleep(Duration::from_secs(5));
}
