//! # replay-rs
//!
//! A Rust library for recording and replaying terminal sessions with timing data,
//! compatible with the classic Unix `script` and `scriptreplay` tools as well as 
//! asciinema's asciicast v2 format. Implemented entirely in Rust with cross-platform support.
//!
//! ## Features
//!
//! - **Record terminal sessions**: Capture command output with precise timing data
//! - **Multiple formats**: Support for both legacy scriptreplay format and modern asciicast v2 format
//! - **Replay with speed control**: Play back sessions at different speeds
//! - **ANSI sequence handling**: Clean up problematic control sequences while preserving colors
//! - **Cross-platform**: Works on macOS, Linux, and other Unix-like systems
//! - **Zero external dependencies**: Built-in implementation, no need for external tools
//! - **Format auto-detection**: Automatically detects file format for seamless playback
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
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Asciicast v2 header containing session metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct AsciinemaHeader {
    version: u8,
    width: u16,
    height: u16,
    timestamp: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<std::collections::HashMap<String, String>>,
}

/// Asciicast v2 event representing terminal output
#[derive(Debug, Serialize, Deserialize)]
pub struct AsciinemaEvent {
    time: f64,
    event_type: String,
    data: String,
}

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
    session_file: String,
}

impl Recorder {
    /// Create a new recorder that will write to the specified files
    ///
    /// # Arguments
    ///
    /// * `output_file` - Path where terminal session output will be written (captured command output)
    /// * `timing_file` - Path where timing data will be written (delays and byte counts for replay)
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
    pub fn record_command(&self, command: Command, plain_text: bool) -> Result<()> {
        self.record_command_format(command, plain_text, false)
    }

    /// Record a command execution in asciicast v2 format
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute and record
    /// * `plain_text` - If true, clean ANSI sequences for better text viewing
    pub fn record_command_asciicast(&self, command: Command, plain_text: bool) -> Result<()> {
        self.record_command_format(command, plain_text, true)
    }

    /// Record a command execution in asciicast v2 format with command metadata
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute and record
    /// * `plain_text` - If true, clean ANSI sequences for better text viewing
    /// * `command_str` - String representation of the command for the header
    pub fn record_command_asciicast_with_metadata(&self, mut command: Command, plain_text: bool, command_str: &str) -> Result<()> {
        // Start the command with pipes
        let child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::inherit()) // Allow user input
            .spawn()
            .map_err(|e| anyhow!("Failed to start command: {}", e))?;

        let start_time = Instant::now();

        // For asciicast format, create single output file
        let mut output_writer = std::fs::File::create(&self.output_file)
            .map_err(|e| anyhow!("Failed to create output file: {}", e))?;

        // Write asciicast v2 header
        let session_start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        // Get actual terminal size, fallback to 80x24 if not available
        let (width, height) = if let Some((terminal_size::Width(w), terminal_size::Height(h))) = terminal_size::terminal_size() {
            (w, h)
        } else {
            (80, 24) // Default fallback
        };

        let header = AsciinemaHeader {
            version: 2,
            width,
            height,
            timestamp: session_start,
            title: Some(format!("Terminal session recorded with replay-rs")),
            command: Some(command_str.to_string()),
            shell: std::env::var("SHELL").ok(),
            env: None,
        };

        writeln!(output_writer, "{}", serde_json::to_string(&header)?)
            .map_err(|e| anyhow!("Failed to write asciicast header: {}", e))?;

        self.record_asciicast(child, output_writer, start_time, plain_text)
    }

    /// Record a command execution with timing data in specified format
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute and record
    /// * `plain_text` - If true, clean ANSI sequences for better text viewing
    /// * `asciicast` - If true, output in asciicast v2 format
    pub fn record_command_format(&self, mut command: Command, plain_text: bool, asciicast: bool) -> Result<()> {
        // Start the command with pipes
        let child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::inherit()) // Allow user input
            .spawn()
            .map_err(|e| anyhow!("Failed to start command: {}", e))?;

        let start_time = Instant::now();

        if asciicast {
            // For asciicast format, create single output file
            let mut output_writer = std::fs::File::create(&self.output_file)
                .map_err(|e| anyhow!("Failed to create output file: {}", e))?;

            // Write asciicast v2 header
            let session_start = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();

            // Get actual terminal size, fallback to 80x24 if not available
            let (width, height) = if let Some((terminal_size::Width(w), terminal_size::Height(h))) = terminal_size::terminal_size() {
                (w, h)
            } else {
                (80, 24) // Default fallback
            };

            let header = AsciinemaHeader {
                version: 2,
                width,
                height,
                timestamp: session_start,
                title: None,
                command: None,
                shell: None,
                env: None,
            };

            writeln!(output_writer, "{}", serde_json::to_string(&header)?)
                .map_err(|e| anyhow!("Failed to write asciicast header: {}", e))?;

            return self.record_asciicast(child, output_writer, start_time, plain_text);
        } else {
            // Create output files for legacy format
            let output_writer = std::fs::File::create(&self.output_file)
                .map_err(|e| anyhow!("Failed to create output file: {}", e))?;
            let timing_writer = std::fs::File::create(&self.timing_file)
                .map_err(|e| anyhow!("Failed to create timing file: {}", e))?;

            return self.record_legacy(child, output_writer, timing_writer, start_time, plain_text);
        }
    }

    fn record_legacy(
        &self,
        mut child: std::process::Child,
        mut output_writer: std::fs::File,
        mut timing_writer: std::fs::File,
        start_time: Instant,
        plain_text: bool,
    ) -> Result<()> {
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

    fn record_asciicast(
        &self,
        mut child: std::process::Child,
        mut output_writer: std::fs::File,
        start_time: Instant,
        plain_text: bool,
    ) -> Result<()> {
        // Handle stdout with byte-level reading for real-time output
        if let Some(mut stdout) = child.stdout.take() {
            let mut buffer = [0u8; 1024];

            loop {
                match stdout.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(bytes_read) => {
                        let now = Instant::now();
                        let time_since_start = now.duration_since(start_time).as_secs_f64();

                        let chunk = &buffer[..bytes_read];
                        let output_data = if plain_text {
                            // For plain text, convert bytes to string and clean up
                            let string_data = String::from_utf8_lossy(chunk);
                            clean_for_display(&string_data)
                        } else {
                            // For binary format, convert bytes to string
                            String::from_utf8_lossy(chunk).to_string()
                        };

                        // Create asciicast event
                        let event = vec![
                            serde_json::Value::Number(serde_json::Number::from_f64(time_since_start).unwrap()),
                            serde_json::Value::String("o".to_string()),
                            serde_json::Value::String(output_data.clone()),
                        ];

                        // Write event as JSON line
                        writeln!(output_writer, "{}", serde_json::to_string(&event)?)
                            .map_err(|e| anyhow!("Failed to write asciicast event: {}", e))?;

                        // Also display to user in real-time
                        print!("{}", output_data);
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
    /// * `timing_file` - Path to the timing data file (empty for asciicast format)
    /// * `session_file` - Path to the recorded terminal output file (contains captured command output)
    pub fn new(timing_file: &str, session_file: &str) -> Result<Self> {
        // Verify session file exists
        if !std::path::Path::new(session_file).exists() {
            return Err(anyhow!("Session file not found: {}", session_file));
        }

        // For legacy format, verify timing file exists
        if !timing_file.is_empty() && !std::path::Path::new(timing_file).exists() {
            return Err(anyhow!("Timing file not found: {}", timing_file));
        }

        Ok(Self {
            timing_file: timing_file.to_string(),
            session_file: session_file.to_string(),
        })
    }

    /// Replay the recorded session
    ///
    /// # Arguments
    ///
    /// * `speed_multiplier` - Playback speed (1.0 = normal, 2.0 = 2x speed, 0.5 = half speed)
    pub fn replay(&self, speed_multiplier: f64) -> Result<()> {
        self.replay_format(speed_multiplier, false)
    }

    /// Replay the recorded session in asciicast format
    ///
    /// # Arguments
    ///
    /// * `speed_multiplier` - Playback speed (1.0 = normal, 2.0 = 2x speed, 0.5 = half speed)
    pub fn replay_asciicast(&self, speed_multiplier: f64) -> Result<()> {
        self.replay_format(speed_multiplier, true)
    }

    /// Replay the recorded session with format detection
    ///
    /// # Arguments
    ///
    /// * `speed_multiplier` - Playback speed (1.0 = normal, 2.0 = 2x speed, 0.5 = half speed)
    /// * `force_asciicast` - Force asciicast format if true, otherwise auto-detect
    pub fn replay_format(&self, speed_multiplier: f64, force_asciicast: bool) -> Result<()> {
        // Detect format if not forced
        let is_asciicast = if force_asciicast {
            true
        } else {
            self.detect_asciicast_format()?
        };

        if is_asciicast {
            self.replay_asciicast_internal(speed_multiplier)
        } else {
            self.replay_legacy_internal(speed_multiplier)
        }
    }

    fn detect_asciicast_format(&self) -> Result<bool> {
        // Try to read the first line to see if it's JSON
        // If it's not valid UTF-8, it's probably a binary file (not asciicast)
        let content = match std::fs::read_to_string(&self.session_file) {
            Ok(content) => content,
            Err(_) => return Ok(false), // Not UTF-8, assume legacy binary format
        };
        
        if let Some(first_line) = content.lines().next() {
            // Try to parse as JSON to detect asciicast header
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(first_line) {
                if let Some(version) = parsed.get("version") {
                    return Ok(version.as_u64() == Some(2));
                }
            }
        }
        
        Ok(false)
    }

    fn replay_legacy_internal(&self, speed_multiplier: f64) -> Result<()> {
        // Read timing file
        let timing_content = std::fs::read_to_string(&self.timing_file)
            .map_err(|e| anyhow!("Failed to read timing file {}: {}", self.timing_file, e))?;

        // Read session file
        let mut session_file = std::fs::File::open(&self.session_file).map_err(|e| {
            anyhow!(
                "Failed to open session file {}: {}",
                self.session_file,
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
            match session_file.read_exact(&mut buffer) {
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
                        return Err(anyhow!("Error reading session file: {}", e));
                    }
                }
            }
        }

        println!();
        Ok(())
    }

    fn replay_asciicast_internal(&self, speed_multiplier: f64) -> Result<()> {
        let content = std::fs::read_to_string(&self.session_file)
            .map_err(|e| anyhow!("Failed to read asciicast file: {}", e))?;

        let mut lines = content.lines();
        
        // Skip the header line
        if lines.next().is_none() {
            return Err(anyhow!("Empty asciicast file"));
        }

        println!("🎬 Playing back asciicast session with replay-rs");
        println!("   Speed: {}x | Press Ctrl+C to stop", speed_multiplier);
        println!();

        let mut last_time = 0.0;

        // Process each event line
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse event line as JSON array
            let event: Vec<serde_json::Value> = serde_json::from_str(line)
                .map_err(|e| anyhow!("Failed to parse asciicast event: {}", e))?;

            if event.len() != 3 {
                continue; // Skip malformed events
            }

            let time = event[0].as_f64()
                .ok_or_else(|| anyhow!("Invalid time in asciicast event"))?;
            let event_type = event[1].as_str()
                .ok_or_else(|| anyhow!("Invalid event type in asciicast event"))?;
            let data = event[2].as_str()
                .ok_or_else(|| anyhow!("Invalid data in asciicast event"))?;

            // Only handle output events
            if event_type != "o" {
                continue;
            }

            // Calculate delay
            let delay = time - last_time;
            last_time = time;

            // Apply speed multiplier and sleep
            let adjusted_delay = delay / speed_multiplier;
            if adjusted_delay >= 0.0001 {
                thread::sleep(Duration::from_secs_f64(adjusted_delay));
            }

            // Output the data
            print!("{}", data);
            std::io::stdout().flush().unwrap_or(());
        }

        println!();
        Ok(())
    }

    /// Replay the session without timing delays (fast dump)
    pub fn dump(&self) -> Result<()> {
        let content = std::fs::read_to_string(&self.session_file)
            .map_err(|e| anyhow!("Failed to read session file: {}", e))?;

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
            .contains("Session file not found"));
    }

    #[test]
    fn test_player_creation_missing_session_file() -> Result<()> {
        // Create only timing file
        let timing_file = "test_timing_only.timing";
        let mut file = File::create(timing_file)?;
        writeln!(file, "0.1 5")?;

        let player = Player::new(timing_file, "nonexistent.out");
        assert!(player.is_err());
        assert!(player
            .unwrap_err()
            .to_string()
            .contains("Session file not found"));

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_player_creation_success() -> Result<()> {
        let timing_file = "test_player_success.timing";
        let session_file = "test_player_success.out";

        // Create both files
        File::create(timing_file)?;
        File::create(session_file)?;

        let player = Player::new(timing_file, session_file);
        assert!(player.is_ok());

        let player = player.unwrap();
        assert_eq!(player.timing_file, timing_file);
        assert_eq!(player.session_file, session_file);

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(session_file).unwrap_or(());
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
        let session_file = "test_dump.out";

        // Create mock timing file
        let mut timing = File::create(timing_file)?;
        writeln!(timing, "0.1 12")?;
        writeln!(timing, "0.5 6")?;

        // Create mock session file
        let mut session = File::create(session_file)?;
        write!(session, "Hello World!")?;
        write!(session, " Test")?;

        let player = Player::new(timing_file, session_file)?;

        // Test dump (should not error)
        player.dump()?;

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(session_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_player_replay_with_mock_data() -> Result<()> {
        let timing_file = "test_replay.timing";
        let session_file = "test_replay.out";

        // Create mock timing file with small delays for fast test
        let mut timing = File::create(timing_file)?;
        writeln!(timing, "0.001 5")?;
        writeln!(timing, "0.001 6")?;

        // Create mock session file
        let mut session = File::create(session_file)?;
        write!(session, "Hello")?;
        write!(session, " Test!")?;

        let player = Player::new(timing_file, session_file)?;

        // Test replay at high speed for fast test
        player.replay(100.0)?; // 100x speed

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(session_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_player_replay_with_invalid_timing() -> Result<()> {
        let timing_file = "test_invalid_timing.timing";
        let session_file = "test_invalid_session.out";

        // Create timing file with invalid data
        let mut timing = File::create(timing_file)?;
        writeln!(timing, "invalid_delay 5")?;
        writeln!(timing, "0.1 invalid_size")?;

        // Create session file
        File::create(session_file)?;

        let player = Player::new(timing_file, session_file)?;

        // Should handle invalid timing gracefully
        let result = player.replay(1.0);
        assert!(result.is_err());

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(session_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_player_replay_different_speeds() -> Result<()> {
        let timing_file = "test_speeds.timing";
        let session_file = "test_speeds.out";

        // Create mock files
        let mut timing = File::create(timing_file)?;
        writeln!(timing, "0.001 4")?;

        let mut session = File::create(session_file)?;
        write!(session, "Fast")?;

        let player = Player::new(timing_file, session_file)?;

        // Test different speeds
        player.replay(0.5)?; // Half speed
        player.replay(2.0)?; // Double speed
        player.replay(10.0)?; // Very fast

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(session_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_player_with_empty_timing_file() -> Result<()> {
        let timing_file = "test_empty_timing.timing";
        let session_file = "test_empty_session.out";

        // Create empty files
        File::create(timing_file)?;
        File::create(session_file)?;

        let player = Player::new(timing_file, session_file)?;

        // Should handle empty timing file gracefully
        player.replay(1.0)?;
        player.dump()?;

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(session_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_player_with_truncated_session() -> Result<()> {
        let timing_file = "test_truncated.timing";
        let session_file = "test_truncated.out";

        // Create timing file expecting more data than session has
        let mut timing = File::create(timing_file)?;
        writeln!(timing, "0.001 100")?; // Expects 100 bytes

        // Create small session file
        let mut session = File::create(session_file)?;
        write!(session, "Short")?; // Only 5 bytes

        let player = Player::new(timing_file, session_file)?;

        // Should handle truncated data gracefully
        let result = player.replay(1.0);
        // This might succeed or fail depending on implementation,
        // but it shouldn't panic
        let _ = result;

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(session_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_recorder_file_creation_errors() {
        use std::os::unix::fs::PermissionsExt;
        
        // Create a directory where we can't write
        let test_dir = "test_no_write_dir";
        fs::create_dir_all(test_dir).unwrap_or(());
        
        // Make directory read-only
        let metadata = fs::metadata(test_dir).unwrap();
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o555); // r-xr-xr-x
        fs::set_permissions(test_dir, permissions).unwrap_or(());
        
        let recorder = Recorder::new(
            &format!("{}/output.log", test_dir),
            &format!("{}/timing.log", test_dir)
        ).unwrap();
        
        let mut cmd = Command::new("echo");
        cmd.arg("test");
        let result = recorder.record_command(cmd, false);
        
        // Should fail due to permission issues
        assert!(result.is_err());
        
        // Restore permissions and clean up
        let mut permissions = fs::metadata(test_dir).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(test_dir, permissions).unwrap_or(());
        fs::remove_dir_all(test_dir).unwrap_or(());
    }

    #[test]
    fn test_player_replay_with_malformed_timing_lines() -> Result<()> {
        let timing_file = "test_malformed.timing";
        let session_file = "test_malformed.out";

        // Create timing file with various malformed lines
        let mut timing = File::create(timing_file)?;
        writeln!(timing, "")?; // Empty line (should be skipped)
        writeln!(timing, "0.1")?; // Missing size (should be skipped)
        writeln!(timing, "0.1 5")?; // Valid line
        writeln!(timing, "   ")?; // Whitespace only (should be skipped)

        // Create session file
        let mut session = File::create(session_file)?;
        write!(session, "Hello")?;

        let player = Player::new(timing_file, session_file)?;
        
        // Should handle malformed lines gracefully
        player.replay(1.0)?;

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(session_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_player_replay_read_error() -> Result<()> {
        let timing_file = "test_read_error.timing";
        let session_file = "test_read_error.out";

        // Create timing file
        let mut timing = File::create(timing_file)?;
        writeln!(timing, "0.1 10")?; // Expects 10 bytes
        writeln!(timing, "0.1 5")?;  // Expects 5 more bytes

        // Create session file with less data than expected
        let mut session = File::create(session_file)?;
        write!(session, "Short")?; // Only 5 bytes

        let player = Player::new(timing_file, session_file)?;
        
        // Should handle EOF gracefully
        let result = player.replay(1.0);
        assert!(result.is_ok()); // Should succeed by breaking on EOF

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(session_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_clean_for_display_edge_cases_2() {
        // Test ESC at end of string
        let input = "Text\x1b";
        let result = clean_for_display(input);
        assert_eq!(result, "Text\x1b");

        // Test incomplete ESC sequence
        let input = "Text\x1b[";
        let result = clean_for_display(input);
        assert_eq!(result, "Text\x1b[");

        // Test ? character not followed by 2004
        let input = "Question? Mark";
        let result = clean_for_display(input);
        assert_eq!(result, "Question? Mark");

        // Test ? at end of string
        let input = "End?";
        let result = clean_for_display(input);
        assert_eq!(result, "End?");

        // Test ?2004 without h or l
        let input = "Test?2004x";
        let result = clean_for_display(input);
        assert_eq!(result, "Test?2004x");
    }

    #[test]
    fn test_recorder_stdout_read_error() {
        // This test simulates a command that closes stdout immediately
        let recorder = Recorder::new("test_stdout_error.log", "test_stdout_error.timing").unwrap();
        
        // Use a command that exits immediately
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("exec 1>&-; exit 0"); // Close stdout and exit
        
        // Should handle the closed stdout gracefully
        let result = recorder.record_command(cmd, false);
        
        // The command technically succeeds (exit 0) even though stdout is closed
        assert!(result.is_ok());
        
        // Clean up
        fs::remove_file("test_stdout_error.log").unwrap_or(());
        fs::remove_file("test_stdout_error.timing").unwrap_or(());
    }

    #[test]
    fn test_recorder_create_timing_file_error() {
        // Create output file first
        let output_file = "test_timing_error.log";
        File::create(output_file).unwrap();
        
        // Create a directory with same name as timing file
        let timing_dir = "test_timing_error.timing";
        fs::create_dir_all(timing_dir).unwrap_or(());
        
        let recorder = Recorder::new(output_file, timing_dir).unwrap();
        
        let mut cmd = Command::new("echo");
        cmd.arg("test");
        let result = recorder.record_command(cmd, false);
        
        // Should fail because timing_dir is a directory, not a file
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to create timing file"));
        
        // Clean up
        fs::remove_file(output_file).unwrap_or(());
        fs::remove_dir_all(timing_dir).unwrap_or(());
    }

    #[test]
    fn test_player_file_open_error() {
        // Create timing file with actual timing data
        let timing_file = "test_player_open_error.timing";
        let session_file = "test_player_open_error.out";
        
        // Create timing file and session file first
        let mut timing = File::create(timing_file).unwrap();
        writeln!(timing, "0.1 5").unwrap();
        drop(timing);
        
        // Create session file
        let mut session = File::create(session_file).unwrap();
        write!(session, "Hello").unwrap();
        drop(session);
        
        // Create player successfully
        let player = Player::new(timing_file, session_file).unwrap();
        
        // Now delete the session file to trigger an error during replay
        fs::remove_file(session_file).unwrap();
        
        let result = player.replay(1.0);
        
        // This should fail because session file was deleted
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to open session file"));
        
        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
    }

    #[test]
    fn test_player_replay_unexpected_error() -> Result<()> {
        let timing_file = "test_unexpected_error.timing";
        let session_file = "test_unexpected_error.out";

        // Create timing file
        let mut timing = File::create(timing_file)?;
        writeln!(timing, "0.1 5")?;
        writeln!(timing, "0.1 10")?; // Request more bytes than available

        // Create session file with limited data
        let mut session = File::create(session_file)?;
        write!(session, "Hello")?; // Only 5 bytes

        let player = Player::new(timing_file, session_file)?;
        
        // This should handle the EOF and continue
        let result = player.replay(1.0);
        assert!(result.is_ok());

        // Clean up
        fs::remove_file(timing_file).unwrap_or(());
        fs::remove_file(session_file).unwrap_or(());
        Ok(())
    }

    #[test]
    fn test_recorder_stdout_read_io_error() {
        // Try to force an I/O error during stdout reading
        let recorder = Recorder::new("test_io_error.log", "test_io_error.timing").unwrap();
        
        // Create a command that produces output then errors
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("echo 'test'; sleep 0.1; kill -9 $$");
        
        // This might fail or succeed depending on timing
        let _ = recorder.record_command(cmd, false);
        
        // Clean up
        fs::remove_file("test_io_error.log").unwrap_or(());
        fs::remove_file("test_io_error.timing").unwrap_or(());
    }

    #[test]
    fn test_clean_for_display_question_mark_edge() {
        // Test ? at position where it could be ?2004 but isn't complete
        let input = "Test?200";
        let result = clean_for_display(input);
        assert_eq!(result, "Test?200");
        
        // Test ? near end without enough chars
        let input = "Test?20";
        let result = clean_for_display(input);
        assert_eq!(result, "Test?20");
    }
}
