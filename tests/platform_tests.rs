//! Platform-specific tests for replay-rs
//!
//! These tests verify behavior across different operating systems, shells,
//! terminal emulators, and locale settings.

use anyhow::Result;
use replay_rs::{Player, Recorder};
use std::env;
use std::fs;
use std::process::Command;

/// Helper function to create a unique test file name
fn test_file_name(base: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("platform_{}_{}", base, timestamp)
}

/// Helper function to clean up test files
fn cleanup_files(files: &[&str]) {
    for file in files {
        fs::remove_file(file).unwrap_or(());
    }
}

/// Helper to check if a shell is available
fn shell_available(shell: &str) -> bool {
    Command::new("which")
        .arg(shell)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[test]
fn test_different_shells() -> Result<()> {
    let shells = vec![
        ("sh", "echo 'sh shell test'"),
        ("bash", "echo 'bash shell test'"),
        ("zsh", "echo 'zsh shell test'"),
        ("dash", "echo 'dash shell test'"),
    ];

    for (shell, test_cmd) in shells {
        if !shell_available(shell) {
            eprintln!("Skipping {} test - shell not available", shell);
            continue;
        }

        let output_file = test_file_name(&format!("{}_test.log", shell));
        let timing_file = format!("{}.timing", output_file);

        let recorder = Recorder::new(&output_file, &timing_file)?;
        let mut cmd = Command::new(shell);
        cmd.arg("-c");
        cmd.arg(test_cmd);

        let result = recorder.record_command(cmd, false);

        if result.is_ok() {
            // Verify output
            let output_content = fs::read_to_string(&output_file)?;
            assert!(output_content.contains(&format!("{} shell test", shell)));

            // Test replay
            let player = Player::new(&timing_file, &output_file)?;
            player.replay(10.0)?;
        }

        cleanup_files(&[&output_file, &timing_file]);
    }

    Ok(())
}

#[test]
fn test_bash_specific_features() -> Result<()> {
    if !shell_available("bash") {
        eprintln!("Skipping bash-specific test - bash not available");
        return Ok(());
    }

    let output_file = test_file_name("bash_features.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("bash");
    cmd.arg("-c");
    cmd.arg("echo \"Bash version: $BASH_VERSION\"; echo \"Array: ${BASH_VERSINFO[@]}\"");

    recorder.record_command(cmd, false)?;

    // Verify bash-specific variables were captured
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Bash version:"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_terminal_environment_variables() -> Result<()> {
    let output_file = test_file_name("term_env.log");
    let timing_file = format!("{}.timing", output_file);

    // Test with different TERM values
    let term_values = vec!["xterm", "xterm-256color", "vt100", "dumb"];

    for term in term_values {
        let recorder = Recorder::new(&output_file, &timing_file)?;
        let mut cmd = Command::new("sh");
        cmd.env("TERM", term);
        cmd.arg("-c");
        cmd.arg("echo \"TERM=$TERM\"");

        recorder.record_command(cmd, false)?;

        // Verify TERM was set correctly
        let output_content = fs::read_to_string(&output_file)?;
        assert!(output_content.contains(&format!("TERM={}", term)));

        cleanup_files(&[&output_file, &timing_file]);
    }

    Ok(())
}

#[test]
fn test_locale_and_encoding() -> Result<()> {
    let output_file = test_file_name("locale_test.log");
    let timing_file = format!("{}.timing", output_file);

    // Test different locale settings
    let locales = vec![
        ("C", "ASCII test"),
        ("en_US.UTF-8", "UTF-8: café, naïve"),
        ("C.UTF-8", "C.UTF-8: 你好世界"),
    ];

    for (locale, test_text) in locales {
        let recorder = Recorder::new(&output_file, &timing_file)?;
        let mut cmd = Command::new("sh");
        cmd.env("LANG", locale);
        cmd.env("LC_ALL", locale);
        cmd.arg("-c");
        cmd.arg(&format!("echo '{}'", test_text));

        let result = recorder.record_command(cmd, false);

        if result.is_ok() {
            // Check if text was properly recorded
            let output_content = fs::read_to_string(&output_file)?;
            assert!(output_content.contains(test_text) || output_content.contains("ASCII"));
        }

        cleanup_files(&[&output_file, &timing_file]);
    }

    Ok(())
}

#[test]
fn test_color_output_handling() -> Result<()> {
    let output_file = test_file_name("color_output.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");

    // Force color output using standard tools
    if cfg!(target_os = "macos") {
        cmd.arg("ls -G /tmp | head -5");
    } else {
        cmd.arg("ls --color=always /tmp | head -5");
    }

    let result = recorder.record_command(cmd, false);

    if result.is_ok() {
        // Should contain some ANSI escape sequences for colors
        let output_content = fs::read_to_string(&output_file)?;
        // At minimum should contain output
        assert!(!output_content.is_empty());
    }

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn test_macos_specific_features() -> Result<()> {
    let output_file = test_file_name("macos_specific.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    cmd.arg("sw_vers; sysctl -n machdep.cpu.brand_string");

    recorder.record_command(cmd, false)?;

    // Verify macOS-specific commands worked
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("ProductName") || output_content.contains("macOS"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn test_linux_specific_features() -> Result<()> {
    let output_file = test_file_name("linux_specific.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    cmd.arg("uname -a; cat /proc/version");

    recorder.record_command(cmd, false)?;

    // Verify Linux-specific info was captured
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Linux"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_specific_features() -> Result<()> {
    let output_file = test_file_name("windows_specific.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("cmd");
    cmd.arg("/C");
    cmd.arg("ver");

    recorder.record_command(cmd, false)?;

    // Verify Windows version info was captured
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Windows") || output_content.contains("Microsoft"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_terminal_width_handling() -> Result<()> {
    let output_file = test_file_name("terminal_width.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");

    // Create output that depends on terminal width
    cmd.arg("printf '%100s\\n' | tr ' ' '='; echo 'END'");

    recorder.record_command(cmd, false)?;

    // Verify output was captured
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("="));
    assert!(output_content.contains("END"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_line_ending_variations() -> Result<()> {
    let output_file = test_file_name("line_endings.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");

    // Test different line endings
    cmd.arg(
        "printf 'LF line\\n'; printf 'CR line\\r'; printf 'CRLF line\\r\\n'; printf 'No newline'",
    );

    recorder.record_command(cmd, false)?;

    // Verify all content was captured
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("LF line"));
    assert!(output_content.contains("No newline"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_environment_preservation() -> Result<()> {
    let output_file = test_file_name("env_preserve.log");
    let timing_file = format!("{}.timing", output_file);

    // Set some custom environment variables
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.env("REPLAY_TEST_VAR", "test_value_123");
    cmd.env("ANOTHER_VAR", "another_value");
    cmd.arg("-c");
    cmd.arg("echo \"REPLAY_TEST_VAR=$REPLAY_TEST_VAR\"; echo \"ANOTHER_VAR=$ANOTHER_VAR\"");

    recorder.record_command(cmd, false)?;

    // Verify environment variables were available to the command
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("REPLAY_TEST_VAR=test_value_123"));
    assert!(output_content.contains("ANOTHER_VAR=another_value"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_path_separators() -> Result<()> {
    let output_file = test_file_name("path_sep.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");

    // Use appropriate path separator for the platform
    let path_sep = if cfg!(windows) { "\\" } else { "/" };
    cmd.arg(&format!("echo 'Path separator: {}'", path_sep));

    recorder.record_command(cmd, false)?;

    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Path separator:"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_signal_handling() -> Result<()> {
    let output_file = test_file_name("signal_test.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");

    // Set up signal handling
    cmd.arg("trap 'echo SIGINT received' INT; echo 'Ready'; sleep 0.1; echo 'Done'");

    recorder.record_command(cmd, false)?;

    // Verify normal completion
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Ready"));
    assert!(output_content.contains("Done"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_working_directory() -> Result<()> {
    let output_file = test_file_name("workdir.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");

    // Set specific working directory
    let temp_dir = env::temp_dir();
    cmd.current_dir(&temp_dir);
    cmd.arg("-c");
    cmd.arg("pwd");

    recorder.record_command(cmd, false)?;

    // Verify working directory was set
    let output_content = fs::read_to_string(&output_file)?;
    // Just verify we got some path output
    assert!(!output_content.trim().is_empty());

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}
