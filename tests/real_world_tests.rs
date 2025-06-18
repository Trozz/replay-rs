//! Real-world scenario tests for replay-rs
//!
//! These tests simulate actual use cases that users might encounter,
//! including development workflows, system administration tasks, and interactive applications.

use anyhow::Result;
use replay_rs::{Player, Recorder};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;

/// Helper function to create a unique test file name
fn test_file_name(base: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("realworld_{}_{}", base, timestamp)
}

/// Helper function to clean up test files
fn cleanup_files(files: &[&str]) {
    for file in files {
        fs::remove_file(file).unwrap_or(());
    }
}

/// Helper to check if a command exists
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[test]
fn test_record_git_operations() -> Result<()> {
    if !command_exists("git") {
        eprintln!("Skipping git test - git not available");
        return Ok(());
    }

    let output_file = test_file_name("git_ops.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("git");
    cmd.args(&["--version"]);

    recorder.record_command(cmd, false)?;

    // Verify git output was captured
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("git version"));

    // Test replay
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(5.0)?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_cargo_build_output() -> Result<()> {
    if !command_exists("cargo") {
        eprintln!("Skipping cargo test - cargo not available");
        return Ok(());
    }

    let output_file = test_file_name("cargo_build.log");
    let timing_file = format!("{}.timing", output_file);

    // Create a minimal Rust project
    let temp_dir = test_file_name("cargo_test_project");
    fs::create_dir(&temp_dir)?;

    let cargo_toml = format!("{}/Cargo.toml", temp_dir);
    let mut file = File::create(&cargo_toml)?;
    writeln!(
        file,
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\""
    )?;

    let src_dir = format!("{}/src", temp_dir);
    fs::create_dir(&src_dir)?;
    let main_rs = format!("{}/main.rs", src_dir);
    let mut file = File::create(&main_rs)?;
    writeln!(file, "fn main() {{ println!(\"Hello, world!\"); }}")?;

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&temp_dir);
    cmd.args(&["check", "--color=always"]);

    let _result = recorder.record_command(cmd, false);

    // Clean up temp project
    fs::remove_dir_all(&temp_dir)?;

    // Test completed regardless of success/failure
    // Just verify the files exist
    assert!(Path::new(&output_file).exists());
    assert!(Path::new(&timing_file).exists());

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_colored_output() -> Result<()> {
    let output_file = test_file_name("colored.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");

    // Generate colored output using ANSI escape codes
    cmd.arg(
        "
        echo '\x1b[31mRed text\x1b[0m';
        echo '\x1b[32mGreen text\x1b[0m';
        echo '\x1b[33mYellow text\x1b[0m';
        echo '\x1b[34mBlue text\x1b[0m';
        echo '\x1b[1mBold text\x1b[0m';
        echo '\x1b[4mUnderlined text\x1b[0m';
    ",
    );

    recorder.record_command(cmd, false)?;

    // Verify colors were preserved
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("\x1b[31m")); // Red
    assert!(output_content.contains("\x1b[32m")); // Green
    assert!(output_content.contains("\x1b[1m")); // Bold

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_progress_bar() -> Result<()> {
    let output_file = test_file_name("progress.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");

    // Simulate a progress bar
    cmd.arg(
        "
        for i in $(seq 0 10 100); do
            printf '\\rProgress: [%-10s] %d%%' \"$(printf '#%.0s' $(seq 1 $((i/10))))\" \"$i\";
            sleep 0.1;
        done;
        echo;
    ",
    );

    recorder.record_command(cmd, false)?;

    // Should contain progress updates
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Progress:"));
    assert!(output_content.contains("100%"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_interactive_python() -> Result<()> {
    if !command_exists("python3") && !command_exists("python") {
        eprintln!("Skipping Python test - Python not available");
        return Ok(());
    }

    let output_file = test_file_name("python_interactive.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");

    // Run Python with some simple commands
    let python_cmd = if command_exists("python3") {
        "python3"
    } else {
        "python"
    };
    cmd.arg(&format!(
        "echo 'print(\"Hello from Python\")
print(2 + 2)
exit()' | {}",
        python_cmd
    ));

    recorder.record_command(cmd, false)?;

    // Should contain Python output
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Hello from Python"));
    assert!(output_content.contains("4"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_curl_download() -> Result<()> {
    if !command_exists("curl") {
        eprintln!("Skipping curl test - curl not available");
        return Ok(());
    }

    let output_file = test_file_name("curl_download.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("curl");
    // Use a small, reliable URL
    cmd.args(&["-I", "https://example.com"]);

    let result = recorder.record_command(cmd, false);

    if result.is_ok() {
        // Should contain HTTP headers
        let output_content = fs::read_to_string(&output_file)?;
        assert!(output_content.contains("HTTP") || output_content.contains("200"));
    }

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_make_output() -> Result<()> {
    let output_file = test_file_name("make_output.log");
    let timing_file = format!("{}.timing", output_file);

    // Create a simple Makefile
    let makefile = test_file_name("Makefile");
    let mut file = File::create(&makefile)?;
    writeln!(
        file,
        "all:\n\t@echo \"Building target...\"\n\t@echo \"Compilation complete!\""
    )?;

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("make");
    cmd.arg("-f");
    cmd.arg(&makefile);

    let result = recorder.record_command(cmd, false);

    if result.is_ok() {
        // Should contain make output
        let output_content = fs::read_to_string(&output_file)?;
        assert!(output_content.contains("Building target"));
        assert!(output_content.contains("Compilation complete"));
    }

    cleanup_files(&[&output_file, &timing_file, &makefile]);
    Ok(())
}

#[test]
fn test_record_npm_output() -> Result<()> {
    if !command_exists("npm") {
        eprintln!("Skipping npm test - npm not available");
        return Ok(());
    }

    let output_file = test_file_name("npm_output.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("npm");
    cmd.args(&["--version"]);

    recorder.record_command(cmd, false)?;

    // Should contain npm version
    let output_content = fs::read_to_string(&output_file)?;
    assert!(!output_content.trim().is_empty());

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_docker_output() -> Result<()> {
    if !command_exists("docker") {
        eprintln!("Skipping docker test - docker not available");
        return Ok(());
    }

    let output_file = test_file_name("docker_output.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("docker");
    cmd.args(&["--version"]);

    recorder.record_command(cmd, false)?;

    // Should contain docker version
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Docker") || output_content.contains("docker"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_system_monitoring() -> Result<()> {
    let output_file = test_file_name("system_monitor.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");

    // Simulate system monitoring output
    if cfg!(target_os = "macos") {
        cmd.arg("vm_stat | head -5");
    } else {
        cmd.arg("free -h 2>/dev/null || vmstat 2>/dev/null || echo 'No memory stats available'");
    }

    recorder.record_command(cmd, false)?;

    // Should contain some output
    let output_content = fs::read_to_string(&output_file)?;
    assert!(!output_content.trim().is_empty());

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_database_query_output() -> Result<()> {
    let output_file = test_file_name("db_query.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");

    // Simulate database query output
    cmd.arg(
        "
        echo 'Connected to database...';
        echo '+----+----------+--------+';
        echo '| id | name     | status |';
        echo '+----+----------+--------+';
        echo '| 1  | Alice    | active |';
        echo '| 2  | Bob      | active |';
        echo '| 3  | Charlie  | inactive|';
        echo '+----+----------+--------+';
        echo '3 rows in set (0.02 sec)';
    ",
    );

    recorder.record_command(cmd, false)?;

    // Should contain table output
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Alice"));
    assert!(output_content.contains("rows in set"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_log_tailing() -> Result<()> {
    let output_file = test_file_name("log_tail.log");
    let timing_file = format!("{}.timing", output_file);

    // Create a temporary log file to tail
    let temp_log = test_file_name("temp.log");
    let mut file = File::create(&temp_log)?;
    for i in 1..=10 {
        writeln!(file, "[2024-01-01 12:00:{:02}] Log entry {}", i, i)?;
    }

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("tail");
    cmd.args(&["-5", &temp_log]);

    recorder.record_command(cmd, false)?;

    // Should contain last 5 entries
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Log entry 6"));
    assert!(output_content.contains("Log entry 10"));
    assert!(!output_content.contains("Log entry 5"));

    cleanup_files(&[&output_file, &timing_file, &temp_log]);
    Ok(())
}

#[test]
fn test_record_test_runner_output() -> Result<()> {
    let output_file = test_file_name("test_runner.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");

    // Simulate test runner output
    cmd.arg(
        "
        echo 'Running test suite...';
        echo '';
        echo '\x1b[32m✓\x1b[0m test_addition';
        echo '\x1b[32m✓\x1b[0m test_subtraction';
        echo '\x1b[31m✗\x1b[0m test_division';
        echo '  Error: Division by zero';
        echo '';
        echo 'Tests: 2 passed, 1 failed, 3 total';
        echo 'Time: 0.123s';
    ",
    );

    recorder.record_command(cmd, false)?;

    // Should contain test results
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("test_addition"));
    assert!(output_content.contains("2 passed, 1 failed"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_record_repl_session() -> Result<()> {
    let output_file = test_file_name("repl_session.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");

    // Simulate a REPL session
    cmd.arg(
        "
        echo '> let x = 42';
        echo '42';
        echo '> x * 2';  
        echo '84';
        echo '> console.log(\"Hello, World!\")';
        echo 'Hello, World!';
        echo 'undefined';
        echo '> exit';
    ",
    );

    recorder.record_command(cmd, false)?;

    // Should contain REPL interaction
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("let x = 42"));
    assert!(output_content.contains("84"));
    assert!(output_content.contains("Hello, World!"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}
