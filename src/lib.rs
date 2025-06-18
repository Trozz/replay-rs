//! # replay-rs
//!
//! A Rust library for recording and replaying terminal sessions with timing data,
//! compatible with the classic Unix `script` and `scriptreplay` tools but implemented
//! entirely in Rust with cross-platform support.
//!
//! ## Features
//!
//! - **Record terminal sessions**: Capture command output with precise timing data
//! - **Replay with speed control**: Play back sessions at different speeds (like asciinema)
//! - **ANSI sequence handling**: Clean up problematic control sequences while preserving colors
//! - **Cross-platform**: Works on macOS, Linux, and other Unix-like systems
//! - **Zero external dependencies**: Built-in implementation, no need for external tools
//! - **Multiple formats**: Support for both raw binary and cleaned text output
//!
//! ## Quick Start
//!
//! ```rust
//! use replay_rs::{Recorder, Player};
//! use std::process::Command;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Record a command
//! let mut recorder = Recorder::new("session.log", "session.log.timing")?;
//! let mut cmd = Command::new("echo");
//! cmd.arg("Hello, World!");
//! recorder.record_command(cmd, false)?; // false = binary format, true = text format
//!
//! // Replay the session
//! let player = Player::new("session.log.timing", "session.log")?;
//! player.replay(1.0)?; // 1.0 = normal speed, 2.0 = 2x speed, etc.
//! # Ok(())
//! # }
//! ```

use anyhow::{anyhow, Result};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// A recorder for capturing terminal sessions with timing data
#[derive(Debug)]
pub struct Recorder {
    output_file: String,
    timing_file: String,
}

/// A player for replaying recorded terminal sessions
#[derive(Debug)]
pub struct Player {
    timing_file: String,
    typescript_file: String,
}

impl Recorder {
    /// Create a new recorder that will write to the specified files
    ///
    /// # Arguments
    ///
    /// * `output_file` - Path where session output will be written
    /// * `timing_file` - Path where timing data will be written
    pub fn new(output_file: &str, timing_file: &str) -> Result<Self> {
        Ok(Self {
            output_file: output_file.to_string(),
            timing_file: timing_file.to_string(),
        })
    }

    /// Record a command execution with timing data
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute and record
    /// * `plain_text` - If true, clean ANSI sequences for better text viewing
    pub fn record_command(&self, mut command: Command, plain_text: bool) -> Result<()> {
        // Start the command with pipes
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::inherit()) // Allow user input
            .spawn()
            .map_err(|e| anyhow!("Failed to start command: {}", e))?;

        // Create output files
        let mut output_writer = std::fs::File::create(&self.output_file)
            .map_err(|e| anyhow!("Failed to create output file: {}", e))?;
        let mut timing_writer = std::fs::File::create(&self.timing_file)
            .map_err(|e| anyhow!("Failed to create timing file: {}", e))?;

        let start_time = Instant::now();
        let mut last_output_time = start_time;

        // Handle stdout with byte-level reading for real-time output
        if let Some(mut stdout) = child.stdout.take() {
            let mut buffer = [0u8; 1024];

            loop {
                match stdout.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(bytes_read) => {
                        let now = Instant::now();
                        let delay = now.duration_since(last_output_time).as_secs_f64();
                        last_output_time = now;

                        let chunk = &buffer[..bytes_read];
                        let output_data = if plain_text {
                            // For plain text, convert bytes to string and clean up
                            let string_data = String::from_utf8_lossy(chunk);
                            clean_for_display(&string_data).into_bytes()
                        } else {
                            // For binary format, keep raw bytes
                            chunk.to_vec()
                        };

                        // Write timing info: delay and size
                        writeln!(timing_writer, "{:.6} {}", delay, output_data.len())
                            .map_err(|e| anyhow!("Failed to write timing data: {}", e))?;

                        // Write output
                        output_writer
                            .write_all(&output_data)
                            .map_err(|e| anyhow!("Failed to write output: {}", e))?;

                        // Also display to user in real-time
                        std::io::stdout().write_all(&output_data).unwrap_or(());
                        std::io::stdout().flush().unwrap_or(());
                    }
                    Err(e) => {
                        eprintln!("Error reading output: {}", e);
                        break;
                    }
                }
            }
        }

        // Wait for the command to complete
        let status = child
            .wait()
            .map_err(|e| anyhow!("Failed to wait for command: {}", e))?;

        if !status.success() {
            return Err(anyhow!(
                "Command failed with exit code: {:?}",
                status.code()
            ));
        }

        Ok(())
    }
}

impl Player {
    /// Create a new player for the specified session files
    ///
    /// # Arguments
    ///
    /// * `timing_file` - Path to the timing data file
    /// * `typescript_file` - Path to the session output file
    pub fn new(timing_file: &str, typescript_file: &str) -> Result<Self> {
        // Verify files exist
        if !std::path::Path::new(timing_file).exists() {
            return Err(anyhow!("Timing file not found: {}", timing_file));
        }
        if !std::path::Path::new(typescript_file).exists() {
            return Err(anyhow!("Typescript file not found: {}", typescript_file));
        }

        Ok(Self {
            timing_file: timing_file.to_string(),
            typescript_file: typescript_file.to_string(),
        })
    }

    /// Replay the recorded session
    ///
    /// # Arguments
    ///
    /// * `speed_multiplier` - Playback speed (1.0 = normal, 2.0 = 2x speed, 0.5 = half speed)
    pub fn replay(&self, speed_multiplier: f64) -> Result<()> {
        // Read timing file
        let timing_content = std::fs::read_to_string(&self.timing_file)
            .map_err(|e| anyhow!("Failed to read timing file {}: {}", self.timing_file, e))?;

        // Read typescript file
        let mut typescript_file = std::fs::File::open(&self.typescript_file).map_err(|e| {
            anyhow!(
                "Failed to open typescript file {}: {}",
                self.typescript_file,
                e
            )
        })?;

        println!("🎬 Playing back session with replay-rs");
        println!("   Speed: {}x | Press Ctrl+C to stop", speed_multiplier);
        println!();

        // Process each timing line
        for line in timing_content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse timing line: "delay size" or just "delay"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue; // Skip malformed lines
            }

            let delay: f64 = parts[0]
                .parse()
                .map_err(|e| anyhow!("Invalid delay value '{}': {}", parts[0], e))?;
            let size: usize = parts[1]
                .parse()
                .map_err(|e| anyhow!("Invalid size value '{}': {}", parts[1], e))?;

            // Apply speed multiplier and skip tiny delays
            let adjusted_delay = delay / speed_multiplier;
            if adjusted_delay >= 0.0001 {
                thread::sleep(Duration::from_secs_f64(adjusted_delay));
            }

            // Read and output the block
            let mut buffer = vec![0u8; size];
            match typescript_file.read_exact(&mut buffer) {
                Ok(_) => {
                    // Output the block
                    print!("{}", String::from_utf8_lossy(&buffer));
                    std::io::stdout().flush().unwrap_or(());
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        // Reached end of file
                        break;
                    } else {
                        return Err(anyhow!("Error reading typescript: {}", e));
                    }
                }
            }
        }

        println!();
        Ok(())
    }

    /// Replay the session without timing delays (fast dump)
    pub fn dump(&self) -> Result<()> {
        let content = std::fs::read_to_string(&self.typescript_file)
            .map_err(|e| anyhow!("Failed to read typescript file: {}", e))?;

        // Clean up only the problematic control sequences but preserve colors
        let cleaned_content = clean_for_display(&content);
        print!("{}", cleaned_content);

        Ok(())
    }
}

/// Clean up problematic ANSI control sequences while preserving colors
///
/// This function removes sequences like bracketed paste mode but keeps
/// color codes and cursor movement sequences that are useful for display.
pub fn clean_for_display(input: &str) -> String {
    let mut result = String::new();
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\x1b' {
            // Handle ESC sequences - preserve color codes but remove problematic ones
            if i + 1 < chars.len() {
                let next_ch = chars[i + 1];
                if next_ch == '[' {
                    // Look ahead to see what kind of sequence this is
                    let mut j = i + 2;
                    let mut sequence = String::new();
                    while j < chars.len() && !chars[j].is_ascii_alphabetic() && chars[j] != '~' {
                        sequence.push(chars[j]);
                        j += 1;
                    }
                    if j < chars.len() {
                        sequence.push(chars[j]);
                    }

                    // Preserve color codes and cursor movement, but skip problematic ones
                    if sequence.contains("2004") {
                        // Skip bracketed paste mode sequences
                        i = j + 1;
                    } else {
                        // Keep the ANSI sequence as-is (for colors, cursor movement, etc.)
                        result.push(ch);
                        i += 1;
                    }
                } else {
                    // Keep other escape sequences
                    result.push(ch);
                    i += 1;
                }
            } else {
                result.push(ch);
                i += 1;
            }
        } else if ch == '?' && i + 5 < chars.len() {
            // Check for bracketed paste mode sequences
            let remaining: String = chars[i + 1..i + 5].iter().collect();
            if remaining == "2004" && (chars[i + 5] == 'h' || chars[i + 5] == 'l') {
                // Skip bracketed paste mode: ?2004h or ?2004l
                i += 6;
            } else {
                result.push(ch);
                i += 1;
            }
        } else if ch.is_control() && ch != '\t' && ch != '\n' && ch != '\r' {
            // Skip other problematic control characters except tab, newline, carriage return
            i += 1;
        } else {
            result.push(ch);
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;

    #[test]
    fn test_clean_for_display() {
        // Test that color codes are preserved
        let input = "\x1b[32mGreen Text\x1b[0m Normal Text";
        let result = clean_for_display(input);
        assert_eq!(result, "\x1b[32mGreen Text\x1b[0m Normal Text");

        // Test that bracketed paste mode is removed
        let input = "?2004hHello\x1b[31m Red\x1b[0m World?2004l";
        let result = clean_for_display(input);
        assert_eq!(result, "Hello\x1b[31m Red\x1b[0m World");

        // Test control character removal but preserve colors
        let input = "\x1b[1;32mBold Green\x1b[0m\x07\x08Text";
        let result = clean_for_display(input);
        assert_eq!(result, "\x1b[1;32mBold Green\x1b[0mText");
    }

    #[test]
    fn test_clean_for_display_edge_cases() {
        // Test empty string
        assert_eq!(clean_for_display(""), "");

        // Test string with only ANSI sequences
        let input = "\x1b[2004h\x1b[0m";
        let result = clean_for_display(input);
        assert_eq!(result, "\x1b[0m");

        // Test tab, newline, and carriage return preservation
        let input = "Line1\tTabbed\nNew Line\rCarriage Return";
        let result = clean_for_display(input);
        assert_eq!(result, "Line1\tTabbed\nNew Line\rCarriage Return");

        // Test multiple bracketed paste sequences
        let input = "?2004hStart?2004lMiddle?2004hEnd?2004l";
        let result = clean_for_display(input);
        assert_eq!(result, "StartMiddleEnd");

        // Test complex ANSI sequence preservation
        let input = "\x1b[1;4;31mBold Underline Red\x1b[0m\x1b[32;40mGreen on Black\x1b[0m";
        let result = clean_for_display(input);
        assert_eq!(
            result,
            "\x1b[1;4;31mBold Underline Red\x1b[0m\x1b[32;40mGreen on Black\x1b[0m"
        );
    }

    #[test]
    fn test_recorder_creation() {
        let recorder = Recorder::new("test.out", "test.timing");
        assert!(recorder.is_ok());

        let recorder = recorder.unwrap();
        assert_eq!(recorder.output_file, "test.out");
        assert_eq!(recorder.timing_file, "test.timing");
    }

    #[test]
    fn test_recorder_with_empty_paths() {
        let recorder = Recorder::new("", "");
        assert!(recorder.is_ok());
    }

    #[test]
    fn test_player_creation_missing_files() {
        let player = Player::new("nonexistent.timing", "nonexistent.out");
        assert!(player.is_err());
        assert!(player
            .unwrap_err()
            .to_string()
            .contains("Timing file not found"));
    }

    #[test]
    fn test_player_creation_missing_typescript() -> Result<()> {
        // Create only timing file
        let timing_file = "test_timing_only.timing";
        let mut file = File::create(timing_file)?;
        writeln!(file, "0.1 5")?;

        let player = Player::new(timing_file, "nonexistent.out");
        assert!(player.is_err());
        assert!(player
            .unwrap_err()
            .to_string()
            .contains("Typescript file not found"));

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_player_creation_success() -> Result<()> {
        let timing_file = "test_player_success.timing";
        let typescript_file = "test_player_success.out";

        // Create both files
        File::create(timing_file)?;
        File::create(typescript_file)?;

        let player = Player::new(timing_file, typescript_file);
        assert!(player.is_ok());

        let player = player.unwrap();
        assert_eq!(player.timing_file, timing_file);
        assert_eq!(player.typescript_file, typescript_file);

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(typescript_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_record_and_replay() -> Result<()> {
        // Create test files
        let output_file = "test_session.log";
        let timing_file = "test_session.timing";

        // Record a simple echo command
        let recorder = Recorder::new(output_file, timing_file)?;
        let mut cmd = std::process::Command::new("echo");
        cmd.arg("Hello, replay-rs!");
        recorder.record_command(cmd, false)?;

        // Verify files were created
        assert!(std::path::Path::new(output_file).exists());
        assert!(std::path::Path::new(timing_file).exists());

        // Verify timing file has content
        let timing_content = fs::read_to_string(timing_file)?;
        assert!(!timing_content.trim().is_empty());

        // Verify output file has content
        let output_content = fs::read_to_string(output_file)?;
        assert!(output_content.contains("Hello, replay-rs!"));

        // Create player and verify it can be created
        let player = Player::new(timing_file, output_file)?;

        // Test dump functionality (faster than full replay in tests)
        player.dump()?;

        // Clean up
        fs::remove_file(output_file).unwrap_or(());
        fs::remove_file(timing_file).unwrap_or(());

        Ok(())
    }

    #[test]
    fn test_record_with_plain_text() -> Result<()> {
        let output_file = "test_plain_session.log";
        let timing_file = "test_plain_session.timing";

        let recorder = Recorder::new(output_file, timing_file)?;
        let mut cmd = std::process::Command::new("echo");
        cmd.arg("Plain text test");
        recorder.record_command(cmd, true)?; // plain_text = true

        // Verify files exist
        assert!(Path::new(output_file).exists());
        assert!(Path::new(timing_file).exists());

        // Clean up
        fs::remove_file(output_file).unwrap_or(());
        fs::remove_file(timing_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_record_failing_command() {
        let output_file = "test_fail_session.log";
        let timing_file = "test_fail_session.timing";

        let recorder = Recorder::new(output_file, timing_file).unwrap();
        let cmd = std::process::Command::new("nonexistent_command_that_should_fail");
        let result = recorder.record_command(cmd, false);

        assert!(result.is_err());

        // Clean up any files that might have been created
        fs::remove_file(output_file).unwrap_or(());
        fs::remove_file(timing_file).unwrap_or(());
    }

    #[test]
    fn test_player_dump_with_mock_data() -> Result<()> {
        let timing_file = "test_dump.timing";
        let typescript_file = "test_dump.out";

        // Create mock timing file
        let mut timing = File::create(timing_file)?;
        writeln!(timing, "0.1 12")?;
        writeln!(timing, "0.5 6")?;

        // Create mock typescript file
        let mut typescript = File::create(typescript_file)?;
        write!(typescript, "Hello World!")?;
        write!(typescript, " Test")?;

        let player = Player::new(timing_file, typescript_file)?;

        // Test dump (should not error)
        player.dump()?;

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(typescript_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_player_replay_with_mock_data() -> Result<()> {
        let timing_file = "test_replay.timing";
        let typescript_file = "test_replay.out";

        // Create mock timing file with small delays for fast test
        let mut timing = File::create(timing_file)?;
        writeln!(timing, "0.001 5")?;
        writeln!(timing, "0.001 6")?;

        // Create mock typescript file
        let mut typescript = File::create(typescript_file)?;
        write!(typescript, "Hello")?;
        write!(typescript, " Test!")?;

        let player = Player::new(timing_file, typescript_file)?;

        // Test replay at high speed for fast test
        player.replay(100.0)?; // 100x speed

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(typescript_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_player_replay_with_invalid_timing() -> Result<()> {
        let timing_file = "test_invalid_timing.timing";
        let typescript_file = "test_invalid_typescript.out";

        // Create timing file with invalid data
        let mut timing = File::create(timing_file)?;
        writeln!(timing, "invalid_delay 5")?;
        writeln!(timing, "0.1 invalid_size")?;

        // Create typescript file
        File::create(typescript_file)?;

        let player = Player::new(timing_file, typescript_file)?;

        // Should handle invalid timing gracefully
        let result = player.replay(1.0);
        assert!(result.is_err());

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(typescript_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_player_replay_different_speeds() -> Result<()> {
        let timing_file = "test_speeds.timing";
        let typescript_file = "test_speeds.out";

        // Create mock files
        let mut timing = File::create(timing_file)?;
        writeln!(timing, "0.001 4")?;

        let mut typescript = File::create(typescript_file)?;
        write!(typescript, "Fast")?;

        let player = Player::new(timing_file, typescript_file)?;

        // Test different speeds
        player.replay(0.5)?; // Half speed
        player.replay(2.0)?; // Double speed
        player.replay(10.0)?; // Very fast

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(typescript_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_player_with_empty_timing_file() -> Result<()> {
        let timing_file = "test_empty_timing.timing";
        let typescript_file = "test_empty_typescript.out";

        // Create empty files
        File::create(timing_file)?;
        File::create(typescript_file)?;

        let player = Player::new(timing_file, typescript_file)?;

        // Should handle empty timing file gracefully
        player.replay(1.0)?;
        player.dump()?;

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(typescript_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_player_with_truncated_typescript() -> Result<()> {
        let timing_file = "test_truncated.timing";
        let typescript_file = "test_truncated.out";

        // Create timing file expecting more data than typescript has
        let mut timing = File::create(timing_file)?;
        writeln!(timing, "0.001 100")?; // Expects 100 bytes

        // Create small typescript file
        let mut typescript = File::create(typescript_file)?;
        write!(typescript, "Short")?; // Only 5 bytes

        let player = Player::new(timing_file, typescript_file)?;

        // Should handle truncated data gracefully
        let result = player.replay(1.0);
        // This might succeed or fail depending on implementation,
        // but it shouldn't panic
        let _ = result;

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(typescript_file).unwrap_or(());
        Ok(())
    }
}
