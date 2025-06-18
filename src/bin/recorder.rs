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
    /// Command to execute and record (defaults to current shell)
    #[arg(value_name = "COMMAND")]
    command: Option<String>,

    /// Arguments for the command
    #[arg(value_name = "ARGS")]
    args: Vec<String>,

    /// Output file for session data
    #[arg(short, long)]
    output: Option<String>,

    /// Timing file for replay data
    #[arg(short, long)]
    timing: Option<String>,

    /// Record in plain text format (removes problematic ANSI sequences)
    #[arg(short, long)]
    plain_text: bool,

    /// Output in asciicast v2 format (single file, asciinema compatible)
    #[arg(short, long)]
    asciicast: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

/// Get the default shell for the current platform
fn get_default_shell() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::var("ComSpec").unwrap_or_else(|_| "powershell".to_string())
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Determine command - use default shell if none specified
    let command = cli.command.unwrap_or_else(get_default_shell);

    // Determine output file name with appropriate extension
    let output_file = cli.output.unwrap_or_else(|| {
        if cli.asciicast {
            "session.cast".to_string()
        } else {
            "session.log".to_string()
        }
    });

    // Determine timing file name (not used for asciicast format)
    let timing_file = if cli.asciicast {
        String::new() // Not used for asciicast
    } else {
        cli.timing
            .unwrap_or_else(|| format!("{}.timing", output_file))
    };

    if cli.verbose {
        println!(
            "📹 Recording command: {} {}",
            command,
            cli.args.join(" ")
        );
        println!("📄 Output file: {}", output_file);
        if !cli.asciicast {
            println!("⏱️  Timing file: {}", timing_file);
        }
        println!(
            "📝 Format: {}",
            if cli.asciicast {
                "Asciicast v2"
            } else if cli.plain_text {
                "Plain text"
            } else {
                "Binary"
            }
        );
        println!();
    }

    // Create the recorder
    let recorder = Recorder::new(&output_file, &timing_file)?;

    // Build the command
    let mut cmd = Command::new(&command);
    cmd.args(&cli.args);

    // Record the command
    println!("🎬 Starting recording...");
    if cli.asciicast {
        // Build command string for metadata
        let mut cmd_parts = vec![command.clone()];
        cmd_parts.extend(cli.args.iter().cloned());
        let command_str = cmd_parts.join(" ");
        
        recorder.record_command_asciicast_with_metadata(cmd, cli.plain_text, &command_str)?;
    } else {
        recorder.record_command(cmd, cli.plain_text)?;
    }

    if cli.verbose {
        println!();
        println!("✅ Recording completed successfully!");
        println!("📂 Files created:");
        println!("   📄 Session: {}", output_file);
        if !cli.asciicast {
            println!("   ⏱️  Timing: {}", timing_file);
        }
        println!();
        println!("🎭 To replay, use:");
        if cli.asciicast {
            println!("   player {} --asciicast", output_file);
        } else {
            println!("   player {} --timing {}", output_file, timing_file);
        }
    } else {
        if cli.asciicast {
            println!("✅ Recording saved to {} (asciicast format)", output_file);
        } else {
            println!(
                "✅ Recording saved to {} (timing: {})",
                output_file, timing_file
            );
        }
    }

    Ok(())
}
