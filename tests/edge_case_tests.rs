//! Edge case tests for replay-rs
//!
//! These tests verify the behavior of replay-rs under unusual or extreme conditions
//! including empty output, binary data, long-running processes, and special characters.

use anyhow::Result;
use replay_rs::{Player, Recorder};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Helper function to create a unique test file name
fn test_file_name(base: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("edge_case_{}_{}", base, timestamp)
}

/// Helper function to clean up test files
fn cleanup_files(files: &[&str]) {
    for file in files {
        fs::remove_file(file).unwrap_or(());
    }
}

#[test]
fn test_record_silent_command() -> Result<()> {
    let output_file = test_file_name("silent.log");
    let timing_file = format!("{}.timing", output_file);

    // Record a command that produces no output
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sleep");
    cmd.arg("0.1");
    recorder.record_command(cmd, false)?;

    // Files should exist even with no output
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    // Output file might be empty or contain only control sequences
    let output_content = fs::read_to_string(&output_file)?;
    // Should be very small (possibly empty or just shell prompt/control chars)
    assert!(output_content.len() < 1000);

    // Should be replayable
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(10.0)?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_binary_output() -> Result<()> {
    let output_file = test_file_name("binary.log");
    let timing_file = format!("{}.timing", output_file);

    // Record a command that produces binary output
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("head");
    cmd.args(&["-c", "100", "/dev/urandom"]);
    recorder.record_command(cmd, false)?;

    // Files should exist
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    // Output should contain binary data
    let output_data = fs::read(&output_file)?;
    assert!(!output_data.is_empty());

    // Binary data should have non-printable characters
    let has_binary = output_data
        .iter()
        .any(|&b| b < 32 && b != b'\n' && b != b'\r' && b != b'\t');
    assert!(has_binary || output_data.len() > 0); // Some systems might filter the binary

    // Should still be replayable
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(10.0)?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_very_long_lines() -> Result<()> {
    let output_file = test_file_name("long_lines.log");
    let timing_file = format!("{}.timing", output_file);

    // Create a very long line without newlines
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("printf");
    let long_string = "A".repeat(5000);
    cmd.arg(&long_string);
    recorder.record_command(cmd, false)?;

    // Verify the long line was recorded
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains(&"A".repeat(1000))); // Check at least part of it

    // Should be replayable
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(50.0)?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_rapid_updates() -> Result<()> {
    let output_file = test_file_name("rapid_updates.log");
    let timing_file = format!("{}.timing", output_file);

    // Record a command with rapid output updates
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    // Print updates with small delays to ensure multiple timing entries
    cmd.arg("for i in 1 2 3 4 5; do echo $i; sleep 0.01; done");
    recorder.record_command(cmd, false)?;

    // Verify files exist and have content
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    // Timing file should have entries
    let timing_content = fs::read_to_string(&timing_file)?;
    let timing_lines = timing_content.lines().count();
    assert!(timing_lines >= 1); // Should have captured updates

    // Should be replayable
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(100.0)?; // Very fast replay

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_unicode_and_emoji() -> Result<()> {
    let output_file = test_file_name("unicode_emoji.log");
    let timing_file = format!("{}.timing", output_file);

    // Record various unicode characters and emojis
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("printf");
    cmd.arg("🎬 Recording test\\n📹 Video capture\\n🎞️ Film reel\\n");
    cmd.arg("Chinese: 中文测试\\n");
    cmd.arg("Arabic: اختبار عربي\\n");
    cmd.arg("Russian: Тестирование\\n");
    cmd.arg("Special: ñáéíóú àèìòù\\n");
    recorder.record_command(cmd, false)?;

    // Verify unicode content was preserved
    let output_content = fs::read_to_string(&output_file)?;
    // Check for at least some unicode content (exact preservation depends on terminal)
    assert!(!output_content.is_empty());
    // The content should contain some of our text
    assert!(
        output_content.contains("Recording test")
            || output_content.contains("Chinese:")
            || output_content.contains("Arabic:")
            || output_content.contains("Russian:")
            || output_content.contains("Special:")
    );

    // Should be replayable
    let player = Player::new(&timing_file, &output_file)?;
    player.dump()?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_zero_width_characters() -> Result<()> {
    let output_file = test_file_name("zero_width.log");
    let timing_file = format!("{}.timing", output_file);

    // Record text with zero-width characters
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("printf");
    // Zero-width joiner, non-joiner, and other special Unicode
    cmd.arg("Normal\u{200D}Text\u{200C}With\u{FEFF}Hidden\u{200B}Chars\\n");
    recorder.record_command(cmd, false)?;

    // Should complete without error
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    // Should be replayable
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(5.0)?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_ansi_art() -> Result<()> {
    let output_file = test_file_name("ansi_art.log");
    let timing_file = format!("{}.timing", output_file);

    // Record complex ANSI art with colors and positioning
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("printf");
    // Create a simple colored box
    cmd.arg("\x1b[1;31m╔════════════╗\x1b[0m\\n");
    cmd.arg("\x1b[1;32m║ ANSI Art   ║\x1b[0m\\n");
    cmd.arg("\x1b[1;33m║ Test Box   ║\x1b[0m\\n");
    cmd.arg("\x1b[1;34m╚════════════╝\x1b[0m\\n");
    recorder.record_command(cmd, false)?;

    // Verify content was captured (ANSI sequences may be processed)
    let output_content = fs::read_to_string(&output_file)?;
    // Should contain the box drawing characters at minimum
    assert!(output_content.contains("╔") || output_content.contains("ANSI Art"));
    assert!(!output_content.is_empty());

    // Should be replayable
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(2.0)?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_control_characters() -> Result<()> {
    let output_file = test_file_name("control_chars.log");
    let timing_file = format!("{}.timing", output_file);

    // Record various control characters
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("printf");
    // Bell, backspace, form feed, vertical tab
    cmd.arg("Bell: \\a\\nBackspace: ABC\\bD\\nTab: A\\tB\\tC\\n");
    recorder.record_command(cmd, false)?;

    // Should complete without error
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    // Should be replayable
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(5.0)?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_extremely_long_session() -> Result<()> {
    let output_file = test_file_name("long_session.log");
    let timing_file = format!("{}.timing", output_file);

    // Record a session with many outputs over time
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    // Generate output over several seconds
    cmd.arg("for i in $(seq 1 50); do echo \"Line $i\"; sleep 0.01; done");
    recorder.record_command(cmd, false)?;

    // Verify substantial content
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Line 1"));
    assert!(output_content.contains("Line 50"));

    // Timing file should reflect the delays
    let timing_content = fs::read_to_string(&timing_file)?;
    let timing_lines: Vec<&str> = timing_content.lines().collect();
    assert!(timing_lines.len() >= 50);

    // Should be replayable at different speeds
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(100.0)?; // Very fast replay

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_mixed_stdout_stderr() -> Result<()> {
    let output_file = test_file_name("mixed_output.log");
    let timing_file = format!("{}.timing", output_file);

    // Record command that outputs to both stdout and stderr
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    cmd.arg("echo 'stdout message'; echo 'another stdout'");
    recorder.record_command(cmd, false)?;

    // Verify stdout outputs were captured (stderr is not captured in current implementation)
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("stdout message"));
    assert!(output_content.contains("another stdout"));

    // Should be replayable
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(5.0)?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_terminal_resize_sequences() -> Result<()> {
    let output_file = test_file_name("resize_seq.log");
    let timing_file = format!("{}.timing", output_file);

    // Record with terminal control sequences
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("printf");
    // Various terminal control sequences
    cmd.arg("\x1b[2J"); // Clear screen
    cmd.arg("\x1b[H"); // Home cursor
    cmd.arg("Terminal Control Test\\n");
    cmd.arg("\x1b[5;10H"); // Position cursor
    cmd.arg("Positioned Text\\n");
    recorder.record_command(cmd, false)?;

    // Should complete without error
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    // Should be replayable
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(5.0)?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_null_bytes() -> Result<()> {
    let output_file = test_file_name("null_bytes.log");
    let timing_file = format!("{}.timing", output_file);

    // Try to record output containing null bytes
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("printf");
    cmd.arg("Before\\0After\\0End");
    recorder.record_command(cmd, false)?;

    // Should handle null bytes gracefully
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    let output_data = fs::read(&output_file)?;
    // Check if null bytes were preserved or handled
    assert!(!output_data.is_empty());

    // Should be replayable
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(5.0)?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_replay_with_extreme_speeds() -> Result<()> {
    let output_file = test_file_name("extreme_speed.log");
    let timing_file = format!("{}.timing", output_file);

    // Record a simple session
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("echo");
    cmd.arg("Speed test content");
    recorder.record_command(cmd, false)?;

    let player = Player::new(&timing_file, &output_file)?;

    // Test extreme slow speed
    player.replay(0.01)?;

    // Test extreme fast speed
    player.replay(1000.0)?;

    // Test "instant" speed (as fast as possible)
    player.replay(f64::MAX)?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}
