use std::process::Command;

fn inspect() -> Command {
    Command::new(env!("CARGO_BIN_EXE_inspect"))
}

#[test]
fn json_lookup_contains_phase_one_fields() {
    let output = inspect()
        .args(["--json", "cp"])
        .output()
        .expect("inspect binary should run");

    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be JSON");

    assert_eq!(value["command"], "cp");
    assert_eq!(value["identity"]["kind"], "external");
    assert!(value["examples"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert!(value["risks"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert!(value["sources"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
}

#[test]
fn missing_command_exits_nonzero() {
    let output = inspect()
        .arg("definitely-not-a-real-command-for-inspect")
        .output()
        .expect("inspect binary should run");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("command not found"));
}

#[test]
fn no_help_suppresses_help_source() {
    let output = inspect()
        .args(["--json", "--no-help", "cp"])
        .output()
        .expect("inspect binary should run");

    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be JSON");

    let sources = value["sources"]
        .as_array()
        .expect("sources should be an array");
    assert!(!sources.iter().any(|source| source == "cp --help"));
}

#[test]
fn cd_is_recognized_as_builtin() {
    let output = inspect()
        .args(["--json", "cd"])
        .output()
        .expect("inspect binary should run");

    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be JSON");

    assert_eq!(value["identity"]["kind"], "builtin");
    assert_eq!(value["summary"], "change the shell working directory");
}

#[test]
fn target_help_flag_is_not_stolen_by_inspect() {
    let output = inspect()
        .args(["cp", "--help"])
        .output()
        .expect("inspect binary should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Command: cp --help"));
    assert!(!stdout.contains("USAGE:\n    inspect"));
}

#[test]
fn json_invocation_mode_uses_leading_inspect_flag() {
    let output = inspect()
        .args(["--json", "cp", "-rn", "src/", "dest/"])
        .output()
        .expect("inspect binary should run");

    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be JSON");

    assert_eq!(value["invocation"], "cp -rn src/ dest/");
    assert_eq!(value["resolved_flags"][0]["typed"], "-r");
}
