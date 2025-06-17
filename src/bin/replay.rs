//! Combined terminal session recorder and player
//! 
//! A unified CLI tool for both recording and replaying terminal sessions.
//! Choose between record and play modes with a simple subcommand interface.

use anyhow::Result;
use clap::{Parser, Subcommand};
use replay_rs::{Player, Recorder};
use std::process::Command;

#[derive(Parser)]
#[command(name = "replay")]
#[command(about = "Record and replay terminal sessions with timing data")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Record a command execution with timing data
    Record {
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
    },
    /// Replay a recorded session with timing data
    Play {
        /// Session file to replay
        #[arg(value_name = "SESSION_FILE")]
        session_file: String,

        /// Timing file for replay data
        #[arg(short, long)]
        timing: Option<String>,

        /// Playback speed multiplier (1.0 = normal, 2.0 = 2x speed, 0.5 = half speed)
        #[arg(short, long, default_value = "1.0")]
        speed: f64,

        /// Fast dump mode (no timing delays, just show content)
        #[arg(short, long)]
        dump: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Record {
            command,
            args,
            output,
            timing,
            plain_text,
            verbose,
        } => {
            // Determine timing file name
            let timing_file = timing.unwrap_or_else(|| format!("{}.timing", output));

            if verbose {
                println!("📹 Recording command: {} {}", command, args.join(" "));
                println!("📄 Output file: {}", output);
                println!("⏱️  Timing file: {}", timing_file);
                println!("📝 Format: {}", if plain_text { "Plain text" } else { "Binary" });
                println!();
            }

            // Create the recorder
            let recorder = Recorder::new(&output, &timing_file)?;

            // Build the command
            let mut cmd = Command::new(&command);
            cmd.args(&args);

            // Record the command
            println!("🎬 Starting recording...");
            recorder.record_command(cmd, plain_text)?;

            if verbose {
                println!();
                println!("✅ Recording completed successfully!");
                println!("📂 Files created:");
                println!("   📄 Session: {}", output);
                println!("   ⏱️  Timing: {}", timing_file);
                println!();
                println!("🎭 To replay, use:");
                println!("   replay play {} --timing {}", output, timing_file);
            } else {
                println!("✅ Recording saved to {} (timing: {})", output, timing_file);
            }
        }
        Commands::Play {
            session_file,
            timing,
            speed,
            dump,
            verbose,
        } => {
            // Determine timing file name
            let timing_file = timing.unwrap_or_else(|| format!("{}.timing", session_file));

            if verbose {
                println!("🎬 Session file: {}", session_file);
                println!("⏱️  Timing file: {}", timing_file);
                if !dump {
                    println!("🚀 Speed: {}x", speed);
                }
                println!("📺 Mode: {}", if dump { "Fast dump" } else { "Timed replay" });
                println!();
            }

            // Create the player
            let player = Player::new(&timing_file, &session_file)?;

            if dump {
                // Fast dump mode
                if verbose {
                    println!("⚡ Fast dumping session content...");
                    println!();
                }
                player.dump()?;
            } else {
                // Timed replay mode
                if verbose {
                    println!("🎭 Starting timed replay...");
                    println!();
                }
                player.replay(speed)?;
            }

            if verbose {
                println!();
                println!("🎊 Playback completed!");
            }
        }
    }

    Ok(())
}