use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use displaydeck_safety::{
    current_tick_ms, digest_label, random_id, ActorFence, ActorStatus, BeginInput, FaultPlan,
    SafetyStatus, TerminalDecision, WatchdogStart,
};

#[test]
fn control_pipe_loss_is_restored_by_independent_actor() {
    let transaction = random_id().unwrap();
    let directory = std::env::temp_dir().join(format!(
        "displaydeck-actor-test-{}-{:02x}",
        std::process::id(),
        transaction[0]
    ));
    fs::create_dir(&directory).unwrap();
    let now = current_tick_ms();
    let start = WatchdogStart {
        schema_version: 1,
        storage_dir: directory.clone(),
        input: BeginInput {
            fence: ActorFence {
                session_id: transaction,
                boot_id: digest_label(b"actor-test-boot"),
                controller_instance_id: random_id().unwrap(),
                watchdog_instance_id: random_id().unwrap(),
                display_id: digest_label(b"actor-test-display"),
                owner_sid_digest: digest_label(b"actor-test-owner"),
                logon_id: 1,
                lease_version: 1,
            },
            machine_epoch: 1,
            now_tick_ms: now,
            confirmation_deadline_tick_ms: now + 5_000,
            previous_display_mode_digest: digest_label(b"actor-test-c0"),
            expected_rollback_snapshot_digest: digest_label(b"actor-test-rollback"),
            candidate_digest: digest_label(b"actor-test-candidate"),
            expected_display_mode_digest: digest_label(b"actor-test-expected"),
        },
        fault_plan: FaultPlan::None,
    };

    let mut child = Command::new(env!("CARGO_BIN_EXE_displaydeck-actor"))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    serde_json::to_writer(&mut stdin, &start).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();

    let status_path = directory.join("status.json");
    let awaiting = wait_for_status(&status_path, Duration::from_secs(3));
    assert_eq!(awaiting.status, SafetyStatus::AwaitingDecision);
    assert_eq!(awaiting.worker_operations_issued, 1);

    drop(stdin);
    let deadline = Instant::now() + Duration::from_secs(3);
    while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let terminal = wait_for_status(&status_path, Duration::from_secs(1));
    assert_eq!(
        terminal.status,
        SafetyStatus::Terminal(TerminalDecision::Reverted)
    );
    assert_eq!(terminal.worker_operations_issued, 2);
    fs::remove_dir_all(directory).unwrap();
}

fn wait_for_status(path: &std::path::Path, timeout: Duration) -> ActorStatus {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(bytes) = fs::read(path) {
            if let Ok(status) = serde_json::from_slice(&bytes) {
                return status;
            }
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for actor status"
        );
        thread::sleep(Duration::from_millis(20));
    }
}
