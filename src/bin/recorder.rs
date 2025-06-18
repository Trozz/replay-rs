//! Terminal session recorder
//!
//! A simple CLI tool for recording terminal sessions with timing data.
//! Records command execution and saves both output and timing information
//! for later replay.

use anyhow::Result;
use clap::Parser;
use replay_rs::Recorder;
use std::process::Command;

#[derive(Parser)]
#[command(name = "recorder")]
#[command(about = "Record terminal sessions with timing data")]
#[command(version = "0.1.0")]
struct Cli {
    /// Command to execute and record
    #[arg(value_name = "COMMAND")]
    command: String,

    /// Arguments for the command
    #[arg(value_name = "ARGS")]
    args: Vec<String>,

    /// Output file for session data
    #[arg(short, long, default_value = "session.log")]
    output: String,

    /// Timing file for replay data
    #[arg(short, long)]
    timing: Option<String>,

    /// Record in plain text format (removes problematic ANSI sequences)
    #[arg(short, long)]
    plain_text: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Determine timing file name
    let timing_file = cli
        .timing
        .unwrap_or_else(|| format!("{}.timing", cli.output));

    if cli.verbose {
        println!(
            "📹 Recording command: {} {}",
            cli.command,
            cli.args.join(" ")
        );
        println!("📄 Output file: {}", cli.output);
        println!("⏱️  Timing file: {}", timing_file);
        println!(
            "📝 Format: {}",
            if cli.plain_text {
                "Plain text"
            } else {
                "Binary"
            }
        );
        println!();
    }

    // Create the recorder
    let recorder = Recorder::new(&cli.output, &timing_file)?;

    // Build the command
    let mut cmd = Command::new(&cli.command);
    cmd.args(&cli.args);

    // Record the command
    println!("🎬 Starting recording...");
    recorder.record_command(cmd, cli.plain_text)?;

    if cli.verbose {
        println!();
        println!("✅ Recording completed successfully!");
        println!("📂 Files created:");
        println!("   📄 Session: {}", cli.output);
        println!("   ⏱️  Timing: {}", timing_file);
        println!();
        println!("🎭 To replay, use:");
        println!("   player {} --timing {}", cli.output, timing_file);
    } else {
        println!(
            "✅ Recording saved to {} (timing: {})",
            cli.output, timing_file
        );
    }

    Ok(())
}
