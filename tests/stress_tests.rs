//! Stress and performance tests for replay-rs
//!
//! These tests verify the behavior of replay-rs under high load conditions,
//! including large outputs, long sessions, and concurrent operations.

use anyhow::Result;
use replay_rs::{Player, Recorder};
use std::fs;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Helper function to create a unique test file name
fn test_file_name(base: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("stress_{}_{}", base, timestamp)
}

/// Helper function to clean up test files
fn cleanup_files(files: &[&str]) {
    for file in files {
        fs::remove_file(file).unwrap_or(());
    }
}

#[test]
fn test_large_output_volume() -> Result<()> {
    let output_file = test_file_name("large_volume.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    // Generate 10,000 lines of output
    cmd.arg("seq 1 10000");
    
    let start = Instant::now();
    recorder.record_command(cmd, false)?;
    let record_duration = start.elapsed();

    // Verify all lines were recorded
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("1"));
    assert!(output_content.contains("10000"));
    
    // Count lines (approximately)
    let line_count = output_content.lines().count();
    assert!(line_count >= 10000);

    // Test replay performance
    let player = Player::new(&timing_file, &output_file)?;
    let start = Instant::now();
    player.replay(100.0)?; // Very fast replay
    let replay_duration = start.elapsed();

    println!("Large output test - Record: {:?}, Replay: {:?}", record_duration, replay_duration);

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_very_long_session_duration() -> Result<()> {
    let output_file = test_file_name("long_duration.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    // Generate output over 5 seconds
    cmd.arg("for i in 1 2 3 4 5; do echo \"Update $i\"; sleep 1; done");
    
    let start = Instant::now();
    recorder.record_command(cmd, false)?;
    let duration = start.elapsed();
    
    // Should take at least 5 seconds
    assert!(duration >= Duration::from_secs(4)); // Allow some margin

    // Verify timing file has appropriate delays
    let timing_content = fs::read_to_string(&timing_file)?;
    let total_delay: f64 = timing_content
        .lines()
        .filter_map(|line| {
            line.split_whitespace()
                .next()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .sum();
    
    // Total delays should be close to actual duration
    assert!(total_delay >= 4.0);

    // Test replay at different speeds
    let player = Player::new(&timing_file, &output_file)?;
    
    // Ultra-fast replay
    let start = Instant::now();
    player.replay(1000.0)?;
    let fast_duration = start.elapsed();
    assert!(fast_duration < Duration::from_secs(1));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_high_frequency_updates() -> Result<()> {
    let output_file = test_file_name("high_freq.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    // Generate 1000 updates as fast as possible
    cmd.arg("for i in $(seq 1 1000); do printf '\\r%04d' $i; done; echo");
    
    recorder.record_command(cmd, false)?;

    // Timing file should have entries
    let timing_content = fs::read_to_string(&timing_file)?;
    let timing_entries = timing_content.lines().count();
    // Should have captured some updates (exact count depends on system performance)
    assert!(timing_entries >= 1);

    // Test replay can handle high-frequency updates
    let player = Player::new(&timing_file, &output_file)?;
    player.replay(50.0)?;

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_memory_usage_large_files() -> Result<()> {
    let output_file = test_file_name("memory_test.log");
    let timing_file = format!("{}.timing", output_file);

    // Generate a large amount of data (1MB+)
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    // Generate ~1MB of data (1000 lines of 1000 'A's each)
    cmd.arg("for i in $(seq 1 1000); do printf '%1000s\\n' | tr ' ' 'A'; done");
    
    recorder.record_command(cmd, false)?;

    // Check file size
    let metadata = fs::metadata(&output_file)?;
    assert!(metadata.len() > 1_000_000); // Should be > 1MB

    // Test that replay can handle large files efficiently
    let player = Player::new(&timing_file, &output_file)?;
    let start = Instant::now();
    player.replay(100.0)?;
    let duration = start.elapsed();
    
    // Should complete reasonably quickly even with large file
    assert!(duration < Duration::from_secs(5));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_concurrent_recording_sessions() -> Result<()> {
    let num_sessions = 10;
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    for i in 0..num_sessions {
        let results = Arc::clone(&results);
        let handle = thread::spawn(move || -> Result<()> {
            let output_file = test_file_name(&format!("concurrent_{}.log", i));
            let timing_file = format!("{}.timing", output_file);

            let recorder = Recorder::new(&output_file, &timing_file)?;
            let mut cmd = Command::new("sh");
            cmd.arg("-c");
            cmd.arg(&format!("echo 'Session {}'; for j in 1 2 3; do echo \"Line $j\"; done", i));
            
            let start = Instant::now();
            recorder.record_command(cmd, false)?;
            let duration = start.elapsed();

            // Store results
            results.lock().unwrap().push((i, duration));

            // Verify content
            let output_content = fs::read_to_string(&output_file)?;
            assert!(output_content.contains(&format!("Session {}", i)));

            cleanup_files(&[&output_file, &timing_file]);
            Ok(())
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap()?;
    }

    // Check results
    let results = results.lock().unwrap();
    assert_eq!(results.len(), num_sessions);

    Ok(())
}

#[test]
fn test_replay_speed_limits() -> Result<()> {
    let output_file = test_file_name("speed_limits.log");
    let timing_file = format!("{}.timing", output_file);

    // Create a session with known timing
    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    cmd.arg("echo 'Start'; sleep 1; echo 'End'");
    
    recorder.record_command(cmd, false)?;

    let player = Player::new(&timing_file, &output_file)?;

    // Test various replay speeds
    let test_speeds = vec![
        (0.1, "Very slow"),
        (0.5, "Half speed"),
        (1.0, "Normal speed"),
        (2.0, "Double speed"),
        (10.0, "10x speed"),
        (100.0, "100x speed"),
        (1000.0, "1000x speed"),
    ];

    for (speed, description) in test_speeds {
        let start = Instant::now();
        player.replay(speed)?;
        let duration = start.elapsed();
        
        println!("Replay at {} ({}): {:?}", speed, description, duration);
        
        // At very high speeds, should complete almost instantly
        if speed >= 100.0 {
            assert!(duration < Duration::from_millis(100));
        }
    }

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_continuous_output_stream() -> Result<()> {
    let output_file = test_file_name("continuous.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    // Simulate continuous output stream
    cmd.arg("for i in $(seq 1 100); do printf '%s' \"$i \"; done; echo");
    
    recorder.record_command(cmd, false)?;

    // Should handle continuous output without newlines
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("1 2 3"));
    assert!(output_content.contains("98 99 100"));

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_burst_output_pattern() -> Result<()> {
    let output_file = test_file_name("burst_output.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    // Create burst pattern: lots of output, pause, lots of output
    cmd.arg("
        for i in $(seq 1 50); do echo \"Burst 1 line $i\"; done;
        sleep 1;
        for i in $(seq 1 50); do echo \"Burst 2 line $i\"; done;
    ");
    
    recorder.record_command(cmd, false)?;

    // Verify both bursts were captured
    let output_content = fs::read_to_string(&output_file)?;
    assert!(output_content.contains("Burst 1 line 50"));
    assert!(output_content.contains("Burst 2 line 50"));

    // Timing should show the pause
    let timing_content = fs::read_to_string(&timing_file)?;
    let max_delay = timing_content
        .lines()
        .filter_map(|line| {
            line.split_whitespace()
                .next()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .fold(0.0f64, |max, delay| max.max(delay));
    
    // Should have at least one significant delay
    assert!(max_delay >= 0.5);

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_many_small_writes() -> Result<()> {
    let output_file = test_file_name("small_writes.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    // Many small writes (one character at a time)
    cmd.arg("for i in $(seq 1 100); do printf '%s' 'X'; done; echo");
    
    recorder.record_command(cmd, false)?;

    // Should capture all characters
    let output_content = fs::read_to_string(&output_file)?;
    let x_count = output_content.matches('X').count();
    assert_eq!(x_count, 100);

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}

#[test]
fn test_resource_cleanup() -> Result<()> {
    // Test that resources are properly cleaned up after many operations
    let mut files_to_cleanup = Vec::new();

    for i in 0..20 {
        let output_file = test_file_name(&format!("cleanup_{}.log", i));
        let timing_file = format!("{}.timing", output_file);
        
        files_to_cleanup.push(output_file.clone());
        files_to_cleanup.push(timing_file.clone());

        // Record
        let recorder = Recorder::new(&output_file, &timing_file)?;
        let mut cmd = Command::new("echo");
        cmd.arg(format!("Test {}", i));
        recorder.record_command(cmd, false)?;

        // Play
        let player = Player::new(&timing_file, &output_file)?;
        player.replay(10.0)?;
        player.dump()?;
        
        // Resources should be released after drop
    }

    // All operations should complete without resource exhaustion
    assert_eq!(files_to_cleanup.len(), 40); // 20 pairs

    // Cleanup
    for file in &files_to_cleanup {
        fs::remove_file(file).unwrap_or(());
    }

    Ok(())
}

#[test]
fn test_extreme_timing_precision() -> Result<()> {
    let output_file = test_file_name("timing_precision.log");
    let timing_file = format!("{}.timing", output_file);

    let recorder = Recorder::new(&output_file, &timing_file)?;
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    // Generate output with very small delays
    cmd.arg("for i in 1 2 3 4 5; do echo $i; sleep 0.001; done");
    
    recorder.record_command(cmd, false)?;

    // Check timing precision
    let timing_content = fs::read_to_string(&timing_file)?;
    let delays: Vec<f64> = timing_content
        .lines()
        .filter_map(|line| {
            line.split_whitespace()
                .next()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .collect();

    // Should have captured timing with reasonable precision
    assert!(!delays.is_empty());
    
    // At least some delays should be non-zero
    let non_zero_delays = delays.iter().filter(|&&d| d > 0.0).count();
    assert!(non_zero_delays > 0);

    cleanup_files(&[&output_file, &timing_file]);
    Ok(())
}