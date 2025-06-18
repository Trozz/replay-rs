//! Tests for example programs in replay-rs
//!
//! These tests verify that the example programs compile correctly and
//! function as expected, ensuring they remain valid demonstrations
//! of the library's capabilities.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

/// Helper function to create unique test file names
#[allow(dead_code)]
fn test_file_name(base: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}_{}", base, timestamp)
}

/// Helper function to clean up test files
#[allow(dead_code)]
fn cleanup_files(files: &[&str]) {
    for file in files {
        fs::remove_file(file).unwrap_or(());
    }
}

/// Get the path to a compiled example
fn example_path(name: &str) -> String {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test executable name
    if path.ends_with("deps") {
        path.pop(); // Remove deps directory
    }
    path.push("examples");
    path.push(name);
    path.to_string_lossy().to_string()
}

#[test]
fn test_simple_record_replay_example() {
    // Run the simple_record_replay example
    let output = Command::new(example_path("simple_record_replay"))
        .output()
        .expect("Failed to execute simple_record_replay example");

    // Check that the example ran successfully
    if !output.status.success() {
        eprintln!(
            "Example failed with stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        eprintln!(
            "Example stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    assert!(output.status.success());

    // Check expected output messages
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Recording a simple command"));
    assert!(stdout.contains("Recording complete"));
    assert!(stdout.contains("Now replaying the session"));
    assert!(stdout.contains("Replay complete"));
    assert!(stdout.contains("Hello from replay-rs!"));
    assert!(stdout.contains("This is a recorded session"));
}

#[test]
fn test_simple_record_replay_no_leftover_files() {
    // The example should clean up after itself
    let example_files = ["example_session.log", "example_session.timing"];

    // Make sure files don't exist before running
    for file in &example_files {
        fs::remove_file(file).unwrap_or(());
    }

    // Run the example
    let output = Command::new(example_path("simple_record_replay"))
        .output()
        .expect("Failed to execute simple_record_replay example");

    assert!(output.status.success());

    // Check that the example cleaned up its files
    for file in &example_files {
        assert!(
            !Path::new(file).exists(),
            "Example left behind file: {}",
            file
        );
    }
}

#[test]
fn test_example_compilation() {
    // Test that examples can be compiled (this is more of a build system test)
    let output = Command::new("cargo")
        .args(&["build", "--examples"])
        .current_dir(".")
        .output()
        .expect("Failed to run cargo build --examples");

    if !output.status.success() {
        eprintln!(
            "Failed to compile examples: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(output.status.success());
}

#[test]
fn test_example_runs_multiple_times() {
    // Test that the example can be run multiple times without interference
    for i in 0..3 {
        let output = Command::new(example_path("simple_record_replay"))
            .output()
            .expect(&format!("Failed to execute example on iteration {}", i));

        assert!(
            output.status.success(),
            "Example failed on iteration {}: {}",
            i,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Hello from replay-rs!"));
    }
}

#[test]
fn test_example_error_handling() {
    // Test that the example handles errors gracefully
    // We can't easily simulate errors in the example itself,
    // but we can test that it doesn't crash under normal conditions

    let output = Command::new(example_path("simple_record_replay"))
        .output()
        .expect("Failed to execute simple_record_replay example");

    // Should not crash
    assert!(output.status.success());

    // Should not have panic messages in stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panic"));
    assert!(!stderr.contains("thread panicked"));
}

#[test]
fn test_example_timing_accuracy() {
    // Test that the example produces reasonable timing
    use std::time::Instant;

    let start = Instant::now();

    let output = Command::new(example_path("simple_record_replay"))
        .output()
        .expect("Failed to execute simple_record_replay example");

    let duration = start.elapsed();

    assert!(output.status.success());

    // The example should complete in a reasonable time (less than 30 seconds)
    assert!(
        duration.as_secs() < 30,
        "Example took too long to complete: {:?}",
        duration
    );

    // But should take at least some time due to the deliberate delays
    assert!(
        duration.as_millis() > 100,
        "Example completed too quickly: {:?}",
        duration
    );
}

#[test]
fn test_example_output_format() {
    let output = Command::new(example_path("simple_record_replay"))
        .output()
        .expect("Failed to execute simple_record_replay example");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for expected emoji and formatting
    assert!(stdout.contains("🎥")); // Recording emoji
    assert!(stdout.contains("✅")); // Check mark
    assert!(stdout.contains("🎬")); // Movie camera
    assert!(stdout.contains("🎉")); // Party emoji

    // Check that output is properly formatted (has newlines)
    assert!(stdout.contains('\n'));

    // Check that the recorded message appears in the output
    assert!(stdout.contains("Hello from replay-rs!"));
    assert!(stdout.contains("This is a recorded session"));
}

#[test]
fn test_example_library_usage() {
    // This test verifies that the example uses the library correctly
    // by checking the source code exists and contains expected patterns

    let example_source = fs::read_to_string("examples/simple_record_replay.rs")
        .expect("Failed to read example source");

    // Check that it uses the main library components
    assert!(example_source.contains("use replay_rs::{Player, Recorder}"));
    assert!(example_source.contains("Recorder::new"));
    assert!(example_source.contains("Player::new"));
    assert!(example_source.contains("record_command"));
    assert!(example_source.contains("replay"));

    // Check that it has proper error handling
    assert!(example_source.contains("Result<(), Box<dyn std::error::Error>>"));
    assert!(example_source.contains("?"));

    // Check that it cleans up files
    assert!(example_source.contains("remove_file"));
}

#[test]
fn test_example_documentation() {
    // Test that the example has proper documentation
    let example_source = fs::read_to_string("examples/simple_record_replay.rs")
        .expect("Failed to read example source");

    // Should have a doc comment explaining what it does
    assert!(example_source.contains("//!"));
    assert!(example_source.contains("Simple example"));

    // Should have inline comments explaining steps
    assert!(example_source.contains("// Record a command"));
    assert!(example_source.contains("// Replay the session"));
}

#[test]
fn test_example_with_different_commands() {
    // While we can't easily modify the example, we can test that
    // it works consistently across multiple runs

    let results: Vec<_> = (0..3)
        .map(|_| {
            Command::new(example_path("simple_record_replay"))
                .output()
                .expect("Failed to execute example")
        })
        .collect();

    // All runs should succeed
    for (i, result) in results.iter().enumerate() {
        assert!(
            result.status.success(),
            "Run {} failed: {}",
            i,
            String::from_utf8_lossy(&result.stderr)
        );
    }

    // All runs should produce similar output
    let outputs: Vec<String> = results
        .iter()
        .map(|r| String::from_utf8_lossy(&r.stdout).to_string())
        .collect();

    for output in &outputs {
        assert!(output.contains("Hello from replay-rs!"));
        assert!(output.contains("Recording complete"));
        assert!(output.contains("Replay complete"));
    }
}
