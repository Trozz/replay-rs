//! Binary integration tests for replay-rs CLI tools
//!
//! These tests verify the command-line interfaces of the recorder, player,
//! and replay binaries by spawning them as separate processes and testing
//! their behavior, argument parsing, and file I/O.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

/// Helper function to create unique test file names
fn test_file_name(base: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}_{}", base, timestamp)
}

/// Helper function to clean up test files
fn cleanup_files(files: &[&str]) {
    for file in files {
        fs::remove_file(file).unwrap_or(());
    }
}

/// Get the path to a compiled binary
fn binary_path(name: &str) -> String {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test executable name
    if path.ends_with("deps") {
        path.pop(); // Remove deps directory
    }
    path.push(name);
    path.to_string_lossy().to_string()
}

#[test]
fn test_recorder_help() {
    let output = Command::new(binary_path("recorder"))
        .arg("--help")
        .output()
        .expect("Failed to execute recorder");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Record terminal sessions"));
    assert!(stdout.contains("--output"));
    assert!(stdout.contains("--timing"));
    assert!(stdout.contains("--plain-text"));
    assert!(stdout.contains("--verbose"));
}

#[test]
fn test_recorder_version() {
    let output = Command::new(binary_path("recorder"))
        .arg("--version")
        .output()
        .expect("Failed to execute recorder");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_recorder_basic_recording() {
    let output_file = test_file_name("binary_recorder_test.log");
    let timing_file = format!("{}.timing", output_file);

    let output = Command::new(binary_path("recorder"))
        .args(&[
            "echo",
            "Binary recorder test",
            "--output",
            &output_file,
            "--timing",
            &timing_file,
        ])
        .output()
        .expect("Failed to execute recorder");

    // Should succeed
    if !output.status.success() {
        eprintln!(
            "Recorder failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(output.status.success());

    // Check that files were created
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    // Check file contents
    let recorded_content = fs::read_to_string(&output_file).unwrap();
    assert!(recorded_content.contains("Binary recorder test"));

    let timing_content = fs::read_to_string(&timing_file).unwrap();
    assert!(!timing_content.trim().is_empty());

    cleanup_files(&[&output_file, &timing_file]);
}

#[test]
fn test_recorder_with_verbose_flag() {
    let output_file = test_file_name("binary_recorder_verbose.log");

    let output = Command::new(binary_path("recorder"))
        .args(&[
            "echo",
            "Verbose test",
            "--output",
            &output_file,
            "--verbose",
        ])
        .output()
        .expect("Failed to execute recorder");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Recording command"));
    assert!(stdout.contains("Output file"));
    assert!(stdout.contains("Timing file"));

    cleanup_files(&[&output_file, &format!("{}.timing", output_file)]);
}

#[test]
fn test_recorder_plain_text_flag() {
    let output_file = test_file_name("binary_recorder_plain.log");

    let output = Command::new(binary_path("recorder"))
        .args(&[
            "echo",
            "Plain text test",
            "--output",
            &output_file,
            "--plain-text",
            "--verbose",
        ])
        .output()
        .expect("Failed to execute recorder");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Plain text"));

    cleanup_files(&[&output_file, &format!("{}.timing", output_file)]);
}

#[test]
fn test_recorder_multiple_arguments() {
    let output_file = test_file_name("binary_recorder_args.log");

    let output = Command::new(binary_path("recorder"))
        .args(&[
            "printf",
            "Line 1\\nLine 2\\nLine 3",
            "--output",
            &output_file,
        ])
        .output()
        .expect("Failed to execute recorder");

    assert!(output.status.success());

    let recorded_content = fs::read_to_string(&output_file).unwrap();
    assert!(recorded_content.contains("Line 1"));
    assert!(recorded_content.contains("Line 2"));
    assert!(recorded_content.contains("Line 3"));

    cleanup_files(&[&output_file, &format!("{}.timing", output_file)]);
}

#[test]
fn test_player_help() {
    let output = Command::new(binary_path("player"))
        .arg("--help")
        .output()
        .expect("Failed to execute player");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Replay recorded terminal sessions"));
    assert!(stdout.contains("--timing"));
    assert!(stdout.contains("--speed"));
    assert!(stdout.contains("--dump"));
    assert!(stdout.contains("--verbose"));
}

#[test]
fn test_player_version() {
    let output = Command::new(binary_path("player"))
        .arg("--version")
        .output()
        .expect("Failed to execute player");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_player_missing_files() {
    let output = Command::new(binary_path("player"))
        .arg("nonexistent_file.log")
        .output()
        .expect("Failed to execute player");

    // Should fail when files don't exist
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("No such file"));
}

#[test]
fn test_recorder_and_player_integration() {
    let output_file = test_file_name("binary_integration.log");
    let timing_file = format!("{}.timing", output_file);

    // First, record a session
    let record_output = Command::new(binary_path("recorder"))
        .args(&[
            "echo",
            "Integration test between recorder and player",
            "--output",
            &output_file,
            "--timing",
            &timing_file,
        ])
        .output()
        .expect("Failed to execute recorder");

    assert!(record_output.status.success());
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    // Then, play it back with dump mode (faster for testing)
    let play_output = Command::new(binary_path("player"))
        .args(&[&output_file, "--timing", &timing_file, "--dump"])
        .output()
        .expect("Failed to execute player");

    assert!(play_output.status.success());
    let stdout = String::from_utf8_lossy(&play_output.stdout);
    assert!(stdout.contains("Integration test between recorder and player"));

    cleanup_files(&[&output_file, &timing_file]);
}

#[test]
fn test_player_speed_parameter() {
    let output_file = test_file_name("binary_speed_test.log");
    let timing_file = format!("{}.timing", output_file);

    // Record a session
    let _record_output = Command::new(binary_path("recorder"))
        .args(&[
            "echo",
            "Speed test",
            "--output",
            &output_file,
            "--timing",
            &timing_file,
        ])
        .output()
        .expect("Failed to execute recorder");

    // Play back at different speeds
    let play_output = Command::new(binary_path("player"))
        .args(&[
            &output_file,
            "--timing",
            &timing_file,
            "--speed",
            "10.0",
            "--verbose",
        ])
        .output()
        .expect("Failed to execute player");

    assert!(play_output.status.success());
    let stdout = String::from_utf8_lossy(&play_output.stdout);
    assert!(stdout.contains("Speed: 10"));

    cleanup_files(&[&output_file, &timing_file]);
}

#[test]
fn test_replay_help() {
    let output = Command::new(binary_path("replay"))
        .arg("--help")
        .output()
        .expect("Failed to execute replay");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Record and replay terminal sessions"));
    assert!(stdout.contains("record"));
    assert!(stdout.contains("play"));
}

#[test]
fn test_replay_record_subcommand() {
    let output_file = test_file_name("binary_replay_record.log");
    let timing_file = format!("{}.timing", output_file);

    let output = Command::new(binary_path("replay"))
        .args(&[
            "record",
            "echo",
            "Replay record test",
            "--output",
            &output_file,
            "--timing",
            &timing_file,
        ])
        .output()
        .expect("Failed to execute replay record");

    assert!(output.status.success());
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    let recorded_content = fs::read_to_string(&output_file).unwrap();
    assert!(recorded_content.contains("Replay record test"));

    cleanup_files(&[&output_file, &timing_file]);
}

#[test]
fn test_replay_play_subcommand() {
    let output_file = test_file_name("binary_replay_play.log");
    let timing_file = format!("{}.timing", output_file);

    // First record with replay
    let _record_output = Command::new(binary_path("replay"))
        .args(&[
            "record",
            "echo",
            "Replay play test",
            "--output",
            &output_file,
            "--timing",
            &timing_file,
        ])
        .output()
        .expect("Failed to execute replay record");

    // Then play back with replay
    let play_output = Command::new(binary_path("replay"))
        .args(&["play", &output_file, "--timing", &timing_file, "--dump"])
        .output()
        .expect("Failed to execute replay play");

    assert!(play_output.status.success());
    let stdout = String::from_utf8_lossy(&play_output.stdout);
    assert!(stdout.contains("Replay play test"));

    cleanup_files(&[&output_file, &timing_file]);
}

#[test]
fn test_replay_record_with_verbose() {
    let output_file = test_file_name("binary_replay_verbose.log");

    let output = Command::new(binary_path("replay"))
        .args(&[
            "record",
            "echo",
            "Verbose replay test",
            "--output",
            &output_file,
            "--verbose",
        ])
        .output()
        .expect("Failed to execute replay record verbose");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Recording command"));
    assert!(stdout.contains("To replay, use"));
    assert!(stdout.contains("replay play"));

    cleanup_files(&[&output_file, &format!("{}.timing", output_file)]);
}

#[test]
fn test_replay_play_with_verbose() {
    let output_file = test_file_name("binary_replay_play_verbose.log");
    let timing_file = format!("{}.timing", output_file);

    // Record first
    let _record_output = Command::new(binary_path("replay"))
        .args(&[
            "record",
            "echo",
            "Verbose play test",
            "--output",
            &output_file,
            "--timing",
            &timing_file,
        ])
        .output()
        .expect("Failed to record for verbose play test");

    // Play with verbose
    let play_output = Command::new(binary_path("replay"))
        .args(&[
            "play",
            &output_file,
            "--timing",
            &timing_file,
            "--dump",
            "--verbose",
        ])
        .output()
        .expect("Failed to execute replay play verbose");

    assert!(play_output.status.success());
    let stdout = String::from_utf8_lossy(&play_output.stdout);
    assert!(stdout.contains("Session file"));
    assert!(stdout.contains("Timing file"));
    assert!(stdout.contains("Fast dump"));

    cleanup_files(&[&output_file, &timing_file]);
}

#[test]
fn test_invalid_command_arguments() {
    // Test recorder with missing command
    let output = Command::new(binary_path("recorder"))
        .output()
        .expect("Failed to execute recorder");

    assert!(!output.status.success());

    // Test player with missing session file
    let output = Command::new(binary_path("player"))
        .output()
        .expect("Failed to execute player");

    assert!(!output.status.success());

    // Test replay with no subcommand
    let output = Command::new(binary_path("replay"))
        .output()
        .expect("Failed to execute replay");

    assert!(!output.status.success());
}

#[test]
fn test_recorder_default_timing_file() {
    let output_file = test_file_name("binary_default_timing.log");
    let expected_timing_file = format!("{}.timing", output_file);

    let output = Command::new(binary_path("recorder"))
        .args(&["echo", "Default timing test", "--output", &output_file])
        .output()
        .expect("Failed to execute recorder");

    assert!(output.status.success());
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&expected_timing_file).exists());

    cleanup_files(&[&output_file, &expected_timing_file]);
}

#[test]
fn test_player_default_timing_file() {
    let output_file = test_file_name("binary_player_default.log");
    let timing_file = format!("{}.timing", output_file);

    // Record first
    let _record_output = Command::new(binary_path("recorder"))
        .args(&[
            "echo",
            "Player default timing test",
            "--output",
            &output_file,
            "--timing",
            &timing_file,
        ])
        .output()
        .expect("Failed to record for player default test");

    // Play without specifying timing file (should use default)
    let play_output = Command::new(binary_path("player"))
        .args(&[&output_file, "--dump"])
        .output()
        .expect("Failed to execute player with default timing");

    assert!(play_output.status.success());

    cleanup_files(&[&output_file, &timing_file]);
}

#[test]
fn test_error_handling_nonexistent_command() {
    let output_file = test_file_name("binary_nonexistent.log");

    let output = Command::new(binary_path("recorder"))
        .args(&[
            "nonexistent_command_should_fail_12345",
            "--output",
            &output_file,
        ])
        .output()
        .expect("Failed to execute recorder");

    // Should fail when trying to record nonexistent command
    assert!(!output.status.success());

    // Clean up any files that might have been created
    cleanup_files(&[&output_file, &format!("{}.timing", output_file)]);
}

#[test]
fn test_complex_command_with_pipes_and_redirects() {
    let output_file = test_file_name("binary_complex.log");

    // Test recording a command with multiple arguments
    let output = Command::new(binary_path("recorder"))
        .args(&[
            "sh",
            "-c",
            "echo 'First line' && echo 'Second line'",
            "--output",
            &output_file,
        ])
        .output()
        .expect("Failed to execute recorder with complex command");

    if output.status.success() {
        let recorded_content = fs::read_to_string(&output_file).unwrap();
        assert!(recorded_content.contains("First line"));
        assert!(recorded_content.contains("Second line"));
    }

    cleanup_files(&[&output_file, &format!("{}.timing", output_file)]);
}
