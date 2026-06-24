//! Scenario coverage: spawn-field diffing and backoff. Adopt-or-spawn in adopt_or_spawn.rs.

use std::time::Duration;

use process_launch::{spawn_fields_differ, BackoffPolicy, SpawnSpec};

/// A change to `command` (a spawn-affecting field) triggers a restart.
#[test]
fn spawn_affecting_change_triggers_restart_command() {
    let a = SpawnSpec {
        command: "daemon-v1".into(),
        args: vec!["--serve".into()],
        cwd: Some("/work".into()),
        ..Default::default()
    };
    let mut b = a.clone();
    b.command = "daemon-v2".into();

    assert!(
        spawn_fields_differ(&a, &b, &[]),
        "a command change is spawn-affecting and triggers a restart"
    );
}

/// A change to `args` (spawn-affecting) triggers a restart.
#[test]
fn spawn_affecting_change_triggers_restart_args() {
    let a = SpawnSpec {
        command: "daemon".into(),
        args: vec!["--serve".into()],
        ..Default::default()
    };
    let mut b = a.clone();
    b.args = vec!["--serve".into(), "--verbose".into()];

    assert!(
        spawn_fields_differ(&a, &b, &[]),
        "an args change is spawn-affecting and triggers a restart"
    );
}

/// A change to `cwd` (spawn-affecting) triggers a restart.
#[test]
fn spawn_affecting_change_triggers_restart_cwd() {
    let a = SpawnSpec {
        command: "daemon".into(),
        cwd: Some("/old-work".into()),
        ..Default::default()
    };
    let mut b = a.clone();
    b.cwd = Some("/new-work".into());

    assert!(
        spawn_fields_differ(&a, &b, &[]),
        "a cwd change is spawn-affecting and triggers a restart"
    );
}

/// A change to a non-spawn field (`metadata`) does NOT trigger a restart.
#[test]
fn non_spawn_change_does_not_restart_metadata() {
    let a = SpawnSpec {
        command: "daemon".into(),
        ..Default::default()
    };
    let mut b = a.clone();
    b.metadata.insert("label".into(), "blue".into());

    assert!(
        !spawn_fields_differ(&a, &b, &[]),
        "a metadata change is non-spawn-affecting and must not trigger a restart"
    );
}

/// A change to `logging_level` (non-spawn-affecting) does NOT trigger restart.
#[test]
fn non_spawn_change_does_not_restart_logging_level() {
    let a = SpawnSpec {
        command: "daemon".into(),
        ..Default::default()
    };
    let mut b = a.clone();
    b.logging_level = Some("debug".into());

    assert!(
        !spawn_fields_differ(&a, &b, &[]),
        "a logging_level change must not trigger a restart"
    );
}

/// An env-var change outside the allowlist does NOT trigger a restart.
#[test]
fn non_spawn_change_does_not_restart_env_outside_allowlist() {
    let a = SpawnSpec {
        command: "daemon".into(),
        ..Default::default()
    };
    let mut b = a.clone();
    b.env.insert("LOG_LEVEL".into(), "trace".into());

    assert!(
        !spawn_fields_differ(&a, &b, &["TILLERD_DIR"]),
        "an env-var outside the allowlist must not trigger a restart"
    );
}

/// The restart delay grows exponentially but is capped, so a persistently
/// failing child does not spin at zero delay.
#[test]
fn capped_backoff_on_repeated_failure_delay_does_not_exceed_cap() {
    let policy = BackoffPolicy {
        base: Duration::from_millis(100),
        cap: Duration::from_millis(800),
        max_attempts: 10,
    };

    // Simulate many repeated failures: no delay ever exceeds the cap.
    for attempt in 1..=64 {
        let delay = policy.delay_for_attempt(attempt);
        assert!(
            delay <= policy.cap,
            "attempt {attempt}: delay {delay:?} exceeds cap {:?}",
            policy.cap
        );
    }
}

/// The first attempt uses the base delay; subsequent attempts grow until the
/// cap, proving exponential growth before the clamp.
#[test]
fn capped_backoff_grows_exponentially_until_the_cap() {
    let policy = BackoffPolicy {
        base: Duration::from_millis(50),
        cap: Duration::from_millis(400),
        max_attempts: 6,
    };

    assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(50));
    assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(100));
    assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(200));
    assert_eq!(policy.delay_for_attempt(4), Duration::from_millis(400));
    // Attempts beyond the cap stay at the cap.
    assert_eq!(policy.delay_for_attempt(5), Duration::from_millis(400));
    assert_eq!(policy.delay_for_attempt(6), Duration::from_millis(400));
}

/// After max_attempts the policy refuses further restarts, preventing a
/// persistently failing child from spinning indefinitely.
#[test]
fn capped_backoff_stops_retrying_after_max_attempts() {
    let policy = BackoffPolicy {
        base: Duration::from_millis(100),
        cap: Duration::from_secs(30),
        max_attempts: 3,
    };

    assert!(policy.should_retry(0), "retry 0 of 3 allowed");
    assert!(policy.should_retry(2), "retry 2 of 3 allowed");
    assert!(
        !policy.should_retry(3),
        "after max_attempts retries are refused"
    );
    assert!(
        !policy.should_retry(100),
        "well past max_attempts is refused"
    );
}
