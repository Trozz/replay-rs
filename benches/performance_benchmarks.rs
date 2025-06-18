//! Performance benchmarks for replay-rs
//!
//! These benchmarks measure the performance of recording and replaying
//! terminal sessions under various conditions to help identify bottlenecks
//! and track performance regressions.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use replay_rs::{clean_for_display, Player, Recorder};
use std::fs::{self, File};
use std::io::Write;
use std::process::Command;
use std::time::SystemTime;

/// Generate a unique test file name for benchmarks
fn bench_file_name(base: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("bench_{}_{}", base, timestamp)
}

/// Clean up benchmark files
fn cleanup_bench_files(files: &[&str]) {
    for file in files {
        fs::remove_file(file).unwrap_or(());
    }
}

/// Benchmark recording different types of commands
fn bench_recording_commands(c: &mut Criterion) {
    let mut group = c.benchmark_group("recording_commands");

    // Benchmark simple echo command
    group.bench_function("echo_simple", |b| {
        b.iter(|| {
            let output_file = bench_file_name("echo_simple.log");
            let timing_file = format!("{}.timing", output_file);

            let recorder = Recorder::new(&output_file, &timing_file).unwrap();
            let mut cmd = Command::new("echo");
            cmd.arg("Simple benchmark test");

            let result = recorder.record_command(cmd, false);
            cleanup_bench_files(&[&output_file, &timing_file]);
            result.unwrap();
        })
    });

    // Benchmark command with multiple lines of output
    group.bench_function("printf_multiline", |b| {
        b.iter(|| {
            let output_file = bench_file_name("printf_multiline.log");
            let timing_file = format!("{}.timing", output_file);

            let recorder = Recorder::new(&output_file, &timing_file).unwrap();
            let mut cmd = Command::new("printf");
            cmd.arg("Line 1\\nLine 2\\nLine 3\\nLine 4\\nLine 5\\n");

            let result = recorder.record_command(cmd, false);
            cleanup_bench_files(&[&output_file, &timing_file]);
            result.unwrap();
        })
    });

    // Benchmark command with substantial output
    group.bench_function("seq_100", |b| {
        b.iter(|| {
            let output_file = bench_file_name("seq_100.log");
            let timing_file = format!("{}.timing", output_file);

            let recorder = Recorder::new(&output_file, &timing_file).unwrap();
            let mut cmd = Command::new("seq");
            cmd.args(&["1", "100"]);

            let result = recorder.record_command(cmd, false);
            cleanup_bench_files(&[&output_file, &timing_file]);
            result.unwrap();
        })
    });

    // Benchmark binary vs text format recording
    group.bench_function("echo_binary_format", |b| {
        b.iter(|| {
            let output_file = bench_file_name("echo_binary.log");
            let timing_file = format!("{}.timing", output_file);

            let recorder = Recorder::new(&output_file, &timing_file).unwrap();
            let mut cmd = Command::new("echo");
            cmd.arg("Binary format test");

            let result = recorder.record_command(cmd, false); // binary format
            cleanup_bench_files(&[&output_file, &timing_file]);
            result.unwrap();
        })
    });

    group.bench_function("echo_text_format", |b| {
        b.iter(|| {
            let output_file = bench_file_name("echo_text.log");
            let timing_file = format!("{}.timing", output_file);

            let recorder = Recorder::new(&output_file, &timing_file).unwrap();
            let mut cmd = Command::new("echo");
            cmd.arg("Text format test");

            let result = recorder.record_command(cmd, true); // text format
            cleanup_bench_files(&[&output_file, &timing_file]);
            result.unwrap();
        })
    });

    group.finish();
}

/// Benchmark replay performance with different speeds
fn bench_replay_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay_performance");

    // Create a standard test session for replay benchmarks
    let setup_session = || {
        let output_file = bench_file_name("replay_setup.log");
        let timing_file = format!("{}.timing", output_file);

        let recorder = Recorder::new(&output_file, &timing_file).unwrap();
        let mut cmd = Command::new("seq");
        cmd.args(&["1", "50"]);
        recorder.record_command(cmd, false).unwrap();

        (output_file, timing_file)
    };

    // Benchmark dump mode (no timing delays)
    group.bench_function("dump_mode", |b| {
        let (output_file, timing_file) = setup_session();

        b.iter(|| {
            let player = Player::new(&timing_file, &output_file).unwrap();
            player.dump().unwrap();
        });

        cleanup_bench_files(&[&output_file, &timing_file]);
    });

    // Benchmark replay at different speeds
    for speed in [1.0, 10.0, 100.0, 1000.0].iter() {
        group.bench_with_input(
            BenchmarkId::new("replay_speed", speed),
            speed,
            |b, &speed| {
                let (output_file, timing_file) = setup_session();

                b.iter(|| {
                    let player = Player::new(&timing_file, &output_file).unwrap();
                    player.replay(speed).unwrap();
                });

                cleanup_bench_files(&[&output_file, &timing_file]);
            },
        );
    }

    group.finish();
}

/// Benchmark ANSI sequence cleaning performance
fn bench_ansi_cleaning(c: &mut Criterion) {
    let mut group = c.benchmark_group("ansi_cleaning");

    // Create test strings with different ANSI sequence complexities
    let simple_ansi = "\x1b[31mRed text\x1b[0m normal text";
    let complex_ansi = "\x1b[1;4;31mBold Underline Red\x1b[0m\x1b[32;40mGreen on Black\x1b[0m\x1b[?2004h\x1b[K\x1b[?2004l";
    let mixed_content = format!(
        "{}\n{}\n{}",
        simple_ansi, complex_ansi, "Normal text without ANSI"
    );

    // Benchmark simple ANSI sequence cleaning
    group.bench_function("simple_ansi", |b| {
        b.iter(|| {
            black_box(clean_for_display(black_box(simple_ansi)));
        })
    });

    // Benchmark complex ANSI sequence cleaning
    group.bench_function("complex_ansi", |b| {
        b.iter(|| {
            black_box(clean_for_display(black_box(complex_ansi)));
        })
    });

    // Benchmark mixed content cleaning
    group.bench_function("mixed_content", |b| {
        b.iter(|| {
            black_box(clean_for_display(black_box(&mixed_content)));
        })
    });

    // Benchmark cleaning with different input sizes
    for size in [100, 1000, 10000].iter() {
        let large_content = complex_ansi.repeat(*size);
        group.throughput(Throughput::Bytes(large_content.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("large_content", size),
            &large_content,
            |b, content| {
                b.iter(|| {
                    black_box(clean_for_display(black_box(content)));
                })
            },
        );
    }

    group.finish();
}

/// Benchmark file I/O operations
fn bench_file_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_operations");

    // Benchmark timing file parsing
    group.bench_function("parse_timing_file", |b| {
        // Create a test timing file
        let timing_file = bench_file_name("timing_parse.timing");
        let output_file = bench_file_name("output_parse.log");

        let mut timing = File::create(&timing_file).unwrap();
        let mut output = File::create(&output_file).unwrap();

        // Write test data
        for i in 0..100 {
            writeln!(timing, "0.{:03} {}", i % 10, i % 50 + 1).unwrap();
            writeln!(output, "Line {}", i).unwrap();
        }

        b.iter(|| {
            let player = Player::new(&timing_file, &output_file).unwrap();
            // Use dump to test file reading without timing delays
            player.dump().unwrap();
        });

        cleanup_bench_files(&[&timing_file, &output_file]);
    });

    // Benchmark recorder file creation and writing
    group.bench_function("recorder_file_creation", |b| {
        b.iter(|| {
            let output_file = bench_file_name("recorder_creation.log");
            let timing_file = format!("{}.timing", output_file);

            let recorder = Recorder::new(&output_file, &timing_file).unwrap();
            let mut cmd = Command::new("echo");
            cmd.arg("File creation test");

            let result = recorder.record_command(cmd, false);
            cleanup_bench_files(&[&output_file, &timing_file]);
            result.unwrap();
        })
    });

    group.finish();
}

/// Benchmark memory usage patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");

    // Benchmark large output handling
    group.bench_function("large_output_recording", |b| {
        b.iter(|| {
            let output_file = bench_file_name("large_output.log");
            let timing_file = format!("{}.timing", output_file);

            let recorder = Recorder::new(&output_file, &timing_file).unwrap();
            let mut cmd = Command::new("seq");
            cmd.args(&["1", "1000"]); // Generate 1000 lines

            let result = recorder.record_command(cmd, false);
            cleanup_bench_files(&[&output_file, &timing_file]);
            result.unwrap();
        })
    });

    // Benchmark string processing with different sizes
    for size in [1_000, 10_000, 100_000].iter() {
        let test_string = "A".repeat(*size);
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("string_processing", size),
            &test_string,
            |b, string| {
                b.iter(|| {
                    black_box(clean_for_display(black_box(string)));
                })
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");

    // Benchmark multiple concurrent recordings
    group.bench_function("concurrent_recordings", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..4)
                .map(|i| {
                    std::thread::spawn(move || {
                        let output_file = bench_file_name(&format!("concurrent_{}.log", i));
                        let timing_file = format!("{}.timing", output_file);

                        let recorder = Recorder::new(&output_file, &timing_file).unwrap();
                        let mut cmd = Command::new("echo");
                        cmd.arg(&format!("Concurrent test {}", i));

                        let result = recorder.record_command(cmd, false);
                        cleanup_bench_files(&[&output_file, &timing_file]);
                        result.unwrap();
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }
        })
    });

    group.finish();
}

/// Benchmark different session sizes
fn bench_session_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_sizes");

    // Test different session sizes
    for lines in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("record_session_size", lines),
            lines,
            |b, &lines| {
                b.iter(|| {
                    let output_file = bench_file_name(&format!("size_{}.log", lines));
                    let timing_file = format!("{}.timing", output_file);

                    let recorder = Recorder::new(&output_file, &timing_file).unwrap();
                    let mut cmd = Command::new("seq");
                    cmd.args(&["1", &lines.to_string()]);

                    let result = recorder.record_command(cmd, false);
                    cleanup_bench_files(&[&output_file, &timing_file]);
                    result.unwrap();
                })
            },
        );
    }

    group.finish();
}

// Register all benchmark groups
criterion_group!(
    benches,
    bench_recording_commands,
    bench_replay_performance,
    bench_ansi_cleaning,
    bench_file_operations,
    bench_memory_patterns,
    bench_concurrent_operations,
    bench_session_sizes
);

criterion_main!(benches);
