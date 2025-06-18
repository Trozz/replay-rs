//! Integration tests for replay-rs
//!
//! These tests verify the end-to-end functionality of recording and replaying
//! terminal sessions, including real command execution and file I/O operations.

use anyhow::Result;
use replay_rs::{clean_for_display, Player, Recorder};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use std::thread;

/// Helper function to create a unique test file name
fn test_file_name(base: &str) -> String {
    let timestamp = std::time::SystemTime::now()
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

#[test]
fn test_record_and_replay_echo_command() -> Result<()> {
    let output_file = test_file_name("integration_echo.log");
    let timing_file = format!("{}.timing", output_file);

    // Record an echo command
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("echo");
    cmd.arg("Integration test message");
    recorder.record_command(cmd, false)?;

    // Verify files exist and have content
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Integration test message"));

    let timing_content = fs::read_to_string(&timing_file)?;
    assert!(!timing_content.trim().is_empty());
    assert!(timing_content.lines().count() > 0);

    // Test replay
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(10.0)?; // High speed for fast test

    // Test dump
    player.dump()?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_multiline_output() -> Result<()> {
    let output_file = test_file_name("integration_multiline.log");
    let timing_file = format!("{}.timing", output_file);

    // Record a command that produces multiple lines
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("printf");
    cmd.arg("Line 1\\nLine 2\\nLine 3\\n");
    recorder.record_command(cmd, false)?;

    // Verify content
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Line 1"));
    assert!(output_content.contains("Line 2"));
    assert!(output_content.contains("Line 3"));

    // Test replay
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(5.0)?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_with_plain_text_format() -> Result<()> {
    let output_file = test_file_name("integration_plain.log");
    let timing_file = format!("{}.timing", output_file);

    // Record with plain text format
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("echo");
    cmd.arg("Plain text format test");
    recorder.record_command(cmd, true)?; // plain_text = true

    // Verify files exist
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Plain text format test"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_command_with_arguments() -> Result<()> {
    let output_file = test_file_name("integration_args.log");
    let timing_file = format!("{}.timing", output_file);

    // Record printf command with multiple arguments
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("printf");
    cmd.args(&[
        "Arg1: %s\\nArg2: %s\\nArg3: %s\\n",
        "first",
        "second",
        "third",
    ]);
    recorder.record_command(cmd, false)?;

    // Verify files exist and have content
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    let output_content = fs::read_to_string(&output_file)?;
    // Should contain the formatted output
    assert!(!output_content.trim().is_empty());
    assert!(output_content.contains("Arg1: first"));
    assert!(output_content.contains("Arg2: second"));
    assert!(output_content.contains("Arg3: third"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_replay_speed_variations() -> Result<()> {
    let output_file = test_file_name("integration_speed.log");
    let timing_file = format!("{}.timing", output_file);

    // Record a command
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("echo");
    cmd.arg("Speed test");
    recorder.record_command(cmd, false)?;

    let player = Player::new(&timing_file, &output_file)?;

    // Test different speeds
    player.replay(0.1)?; // Very slow (but still fast for testing)
    player.replay(1.0)?; // Normal speed
    player.replay(5.0)?; // Fast
    player.replay(100.0)?; // Very fast

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_failing_command() {
    let output_file = test_file_name("integration_fail.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file).unwrap();

    // Try to record a command that should fail
    let cmd = Command::new("false"); // 'false' command always exits with code 1
    let result = recorder.record_command(cmd, false);

    // Should return an error
    assert!(result.is_err());

    cleanup_files(&[&output_file, &timing_file]);
}

#[test]
fn test_large_output_recording() -> Result<()> {
    let output_file = test_file_name("integration_large.log");
    let timing_file = format!("{}.timing", output_file);

    // Create a command that generates substantial output
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("seq");
    cmd.args(&["1", "100"]); // Generate numbers 1-100
    recorder.record_command(cmd, false)?;

    // Verify substantial content was recorded
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.lines().count() >= 100);
    assert!(output_content.contains("1"));
    assert!(output_content.contains("100"));

    // Test replay
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(50.0)?; // Very fast for testing

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_timing_file_format() -> Result<()> {
    let output_file = test_file_name("integration_timing.log");
    let timing_file = format!("{}.timing", output_file);

    // Record a simple command
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("echo");
    cmd.arg("timing test");
    recorder.record_command(cmd, false)?;

    // Verify timing file format
    let timing_content = fs::read_to_string(&timing_file)?;
    let lines: Vec<&str> = timing_content.lines().collect();

    assert!(!lines.is_empty());

    for line in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            // Should be able to parse delay as float
            let delay: Result<f64, _> = parts[0].parse();
            assert!(delay.is_ok());
            assert!(delay.unwrap() >= 0.0);

            // Should be able to parse size as usize
            let size: Result<usize, _> = parts[1].parse();
            assert!(size.is_ok());
            assert!(size.unwrap() > 0);
        }
    }

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_concurrent_recording() -> Result<()> {
    let handles: Vec<_> = (0..3)
        .map(|i| {
            thread::spawn(move || -> Result<()> {
                let output_file = test_file_name(&format!("integration_concurrent_{}.log", i));
                let timing_file = format!("{}.timing", output_file);

                let recorder = Recorder::new(&output_file, &timing_file)?;
                let mut cmd = Command::new("echo");
                cmd.arg(&format!("Concurrent test {}", i));
                recorder.record_command(cmd, false)?;

                // Verify files exist
                assert!(Path::new(&output_file).exists());
                assert!(Path::new(&timing_file).exists());

                cleanup_files(&[&output_file, &timing_file]);
                Ok(())
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap()?;
    }

    Ok(())
}

#[test]
fn test_ansi_sequence_handling() -> Result<()> {
    // Test the clean_for_display function with various ANSI sequences

    // Basic color codes should be preserved
    let input = "\x1b[31mRed text\x1b[0m normal text";
    let result = clean_for_display(input);
    assert_eq!(result, "\x1b[31mRed text\x1b[0m normal text");

    // Bracketed paste mode should be removed
    let input = "\x1b[?2004htext\x1b[?2004l";
    let result = clean_for_display(input);
    assert_eq!(result, "text");

    // Complex sequences
    let input = "\x1b[1;32mBold Green\x1b[0m\x1b[?2004h\x1b[K\x1b[?2004l";
    let result = clean_for_display(input);
    assert_eq!(result, "\x1b[1;32mBold Green\x1b[0m\x1b[K");

    Ok(())
}

#[test]
fn test_empty_session_handling() -> Result<()> {
    let output_file = test_file_name("integration_empty.log");
    let timing_file = format!("{}.timing", output_file);

    // Create empty session files
    File::create(&output_file)?;
    File::create(&timing_file)?;

    let player = Player::new(&timing_file, &output_file)?;

    // Should handle empty files gracefully
    player.replay(1.0)?;
    player.dump()?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_malformed_timing_file_handling() -> Result<()> {
    let output_file = test_file_name("integration_malformed.log");
    let timing_file = format!("{}.timing", output_file);

    // Create output file with some content
    let mut output = File::create(&output_file)?;
    writeln!(output, "Some output content")?;

    // Create malformed timing file
    let mut timing = File::create(&timing_file)?;
    writeln!(timing, "malformed line")?;
    writeln!(timing, "0.1")?; // Missing size
    writeln!(timing, "not_a_number 5")?;
    writeln!(timing, "0.1 not_a_size")?;
    writeln!(timing, "0.1 5")?; // Valid line

    let player = Player::new(&timing_file, &output_file)?;

    // Should handle malformed timing gracefully
    let result = player.replay(1.0);
    // May succeed or fail, but shouldn't panic
    let _ = result;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_file_permissions_and_cleanup() -> Result<()> {
    let output_file = test_file_name("integration_perms.log");
    let timing_file = format!("{}.timing", output_file);

    // Record a session
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("echo");
    cmd.arg("permissions test");
    recorder.record_command(cmd, false)?;

    // Check that files are readable
    let mut output_content = Vec::new();
    let mut file = File::open(&output_file)?;
    file.read_to_end(&mut output_content)?;
    assert!(!output_content.is_empty());

    let timing_content = fs::read_to_string(&timing_file)?;
    assert!(!timing_content.is_empty());

    // Test that we can replay
    let player = Player::new(&timing_file, &output_file)?;
    player.dump()?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_binary_vs_text_format_differences() -> Result<()> {
    let binary_output = test_file_name("integration_binary.log");
    let binary_timing = format!("{}.timing", binary_output);
    let text_output = test_file_name("integration_text.log");
    let text_timing = format!("{}.timing", text_output);

    // Record same command in both formats
    let test_message = "Format comparison test";

    // Binary format
    let recorder_binary = Recorder::new(&binary_output, &binary_timing)?;
    let mut cmd = Command::new("echo");
    cmd.arg(test_message);
    recorder_binary.record_command(cmd, false)?;

    // Text format
    let recorder_text = Recorder::new(&text_output, &text_timing)?;
    let mut cmd = Command::new("echo");
    cmd.arg(test_message);
    recorder_text.record_command(cmd, true)?;

    // Both should contain the test message
    let binary_content = fs::read_to_string(&binary_output)?;
    let text_content = fs::read_to_string(&text_output)?;

    assert!(binary_content.contains(test_message));
    assert!(text_content.contains(test_message));

    // Both should be playable
    let player_binary = Player::new(&binary_timing, &binary_output)?;
    let player_text = Player::new(&text_timing, &text_output)?;

    player_binary.dump()?;
    player_text.dump()?;

    cleanup_files(&[&binary_output, &binary_timing, &text_output, &text_timing]);
    Ok(())
}

#[test]
fn test_stress_multiple_sessions() -> Result<()> {
    let sessions = 5;
    let mut files_to_cleanup = Vec::new();

    for i in 0..sessions {
        let output_file = test_file_name(&format!("stress_session_{}.log", i));
        let timing_file = format!("{}.timing", output_file);

        files_to_cleanup.push(output_file.clone());
        files_to_cleanup.push(timing_file.clone());

        // Record session
        let recorder = Recorder::new(&output_file, &timing_file)?;
        let mut cmd = Command::new("echo");
        cmd.arg(&format!("Stress test session {}", i));
        recorder.record_command(cmd, false)?;

        // Verify and replay
        assert!(Path::new(&output_file).exists());
        assert!(Path::new(&timing_file).exists());

        let player = Player::new(&timing_file, &output_file)?;
        player.replay(20.0)?; // Fast replay
    }

    // Clean up all files
    for file in &files_to_cleanup {
        fs::remove_file(file).unwrap_or(());
    }

    Ok(())
}

#[test]
fn test_unicode_and_special_characters() -> Result<()> {
    let output_file = test_file_name("integration_unicode.log");
    let timing_file = format!("{}.timing", output_file);

    // Test with unicode and special characters
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("echo");
    cmd.arg("Unicode: 🎬 ñáéíóú ñàèìòù αβγδε 中文字符");
    recorder.record_command(cmd, false)?;

    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("🎬"));
    assert!(output_content.contains("ñáéíóú"));
    assert!(output_content.contains("αβγδε"));

    let player = Player::new(&timing_file, &output_file)?;
    player.dump()?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}
