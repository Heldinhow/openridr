use super::*;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// What `env -0` prints between the markers: NUL-separated `KEY=VALUE` entries.
fn shell_output(entries: &[&str]) -> Vec<u8> {
    let mut bytes = BEGIN.as_bytes().to_vec();
    for entry in entries {
        bytes.extend_from_slice(entry.as_bytes());
        bytes.push(0);
    }
    bytes.extend_from_slice(END.as_bytes());
    bytes
}

#[test]
fn the_parser_reads_nul_separated_entries_between_the_markers() {
    let mut bytes = b"a banner the .zshrc printed\n".to_vec();
    bytes.extend(shell_output(&[
        "PATH=/usr/bin:/bin",
        "GREETING=two\nlines",
        "TOKEN=a=b",
    ]));
    bytes.extend_from_slice(b"trailing noise\n");
    assert_eq!(
        parse(&bytes).unwrap(),
        vec![
            ("PATH".to_string(), "/usr/bin:/bin".to_string()),
            ("GREETING".to_string(), "two\nlines".to_string()),
            ("TOKEN".to_string(), "a=b".to_string()),
        ]
    );
}

#[test]
fn the_parser_keeps_an_empty_value_and_drops_entries_without_a_key() {
    assert_eq!(
        parse(&shell_output(&["EMPTY=", "=novalue", "NOEQUALS", "A=1"])).unwrap(),
        vec![
            ("EMPTY".to_string(), String::new()),
            ("A".to_string(), "1".to_string()),
        ]
    );
}

#[test]
fn the_parser_refuses_output_without_both_markers() {
    assert!(parse(b"PATH=/bin\0").is_none());
    let mut missing_end = BEGIN.as_bytes().to_vec();
    missing_end.extend_from_slice(b"PATH=/bin\0");
    assert!(parse(&missing_end).is_none());
}

#[test]
fn the_parser_drops_an_entry_that_is_not_utf8() {
    let mut bytes = BEGIN.as_bytes().to_vec();
    bytes.extend_from_slice(b"GOOD=1\0BAD=");
    bytes.push(0xff);
    bytes.push(0);
    bytes.extend_from_slice(END.as_bytes());
    assert_eq!(
        parse(&bytes).unwrap(),
        vec![("GOOD".to_string(), "1".to_string())]
    );
}

#[test]
fn merging_skips_the_variables_that_describe_the_helper_shell() {
    let vars: Vec<(String, String)> = ["_", "SHLVL", "PWD", "OLDPWD", "PATH"]
        .iter()
        .map(|key| (key.to_string(), "value".to_string()))
        .collect();
    assert_eq!(
        merge(&vars),
        vec![("PATH".to_string(), "value".to_string())]
    );
}

#[test]
fn resolving_is_macos_only_and_only_when_launchd_started_the_app() {
    // Finder, the Dock and Spotlight: launchd is the parent.
    assert!(should_resolve(true, 1));
    // A terminal launch: the shell is the parent, so the environment is already the user's.
    assert!(!should_resolve(true, 4711));
    // Linux and Windows keep the behavior they had.
    assert!(!should_resolve(false, 1));
}

#[test]
fn the_shell_is_the_users_or_zsh() {
    assert_eq!(shell_path_from(None), PathBuf::from("/bin/zsh"));
    assert_eq!(
        shell_path_from(Some(OsString::from(""))),
        PathBuf::from("/bin/zsh")
    );
    assert_eq!(
        shell_path_from(Some(OsString::from("/bin/bash"))),
        PathBuf::from("/bin/bash")
    );
}

#[cfg(unix)]
#[test]
fn capturing_reads_what_the_child_printed() {
    let mut child = Command::new("/bin/sh")
        .args(["-c", "printf 'A=1\\0B=2'"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let output = capture(&mut child, Duration::from_secs(10)).unwrap();
    assert!(output.status.success());
    assert_eq!(output.bytes, b"A=1\0B=2");
}

#[cfg(unix)]
#[test]
fn capturing_gives_up_on_a_child_that_outlives_the_timeout() {
    let mut child = Command::new("/bin/sh")
        .args(["-c", "sleep 30"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    assert_eq!(
        capture(&mut child, Duration::from_millis(200)).unwrap_err(),
        Failure::Timeout
    );
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "the timeout was not the thing that ended the wait"
    );
    assert!(
        child.try_wait().unwrap().is_some(),
        "the child was not killed and reaped"
    );
}

#[cfg(unix)]
#[test]
fn capturing_does_not_wait_for_a_grandchild_holding_the_pipe() {
    // A `.zshrc` that leaves a daemon behind keeps the pipe open, so EOF never comes: the deadline
    // is what ends the wait instead of the app hanging on the read.
    let mut child = Command::new("/bin/sh")
        .args(["-c", "sleep 0.5 & printf 'A=1'"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    assert_eq!(
        capture(&mut child, Duration::from_millis(100)).unwrap_err(),
        Failure::Timeout
    );
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "the pipe held by the grandchild blocked the wait"
    );
}

#[cfg(unix)]
#[test]
fn resolving_runs_the_shell_and_reads_the_environment_it_prints() {
    let vars = resolve(Path::new("/bin/sh"), Duration::from_secs(20)).unwrap();
    assert!(
        vars.iter()
            .any(|(key, value)| key == "PATH" && !value.is_empty()),
        "no PATH in {vars:?}"
    );
}

#[cfg(unix)]
fn fake_shell(name: &str, body: &str) -> (tempfile::TempDir, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let shell = dir.path().join(name);
    std::fs::write(&shell, body).unwrap();
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).unwrap();
    (dir, shell)
}

#[cfg(unix)]
#[test]
fn resolving_ignores_what_the_shell_prints_around_the_environment() {
    let (_dir, shell) = fake_shell(
        "noisy-shell",
        "#!/bin/sh\nprintf 'welcome to your shell\\n'\nexec /bin/sh \"$@\"\n",
    );
    let vars = resolve(&shell, Duration::from_secs(20)).unwrap();
    assert!(
        vars.iter()
            .any(|(key, value)| key == "PATH" && !value.is_empty()),
        "the banner was parsed instead of the environment: {vars:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_shell_that_prints_no_environment_is_a_named_failure() {
    // `/usr/bin/true` ignores the arguments and prints nothing, which is the same "no markers"
    // path a shell that rejected `env -0` takes.
    let failure = resolve(Path::new("/usr/bin/true"), Duration::from_secs(20)).unwrap_err();
    assert!(
        failure.to_string().contains("printed no environment"),
        "{failure}"
    );
}

#[cfg(unix)]
#[test]
fn a_failure_never_repeats_what_the_shell_printed() {
    // The environment may hold tokens, so a shell that prints one but no markers is reported by its
    // exit status, never by its output.
    let (_dir, shell) = fake_shell(
        "leaky-shell",
        "#!/bin/sh\nprintf 'TOKEN=super-secret\\n'\nexit 0\n",
    );
    let message = match resolve(&shell, Duration::from_secs(20)) {
        Ok(vars) => panic!("the shell printed no markers: {vars:?}"),
        Err(failure) => failure.to_string(),
    };
    assert!(
        !message.contains("super-secret") && !message.contains("TOKEN="),
        "{message}"
    );
}

#[cfg(unix)]
#[test]
fn resolving_reports_a_shell_that_cannot_be_started() {
    let failure = resolve(Path::new("/nonexistent/shell"), Duration::from_secs(1)).unwrap_err();
    assert!(matches!(failure, Failure::Message(_)), "{failure:?}");
}
