//! Terminal session player
//!
//! A simple CLI tool for replaying recorded terminal sessions with timing data.
//! Supports speed control and different playback modes.

use anyhow::Result;
use clap::Parser;
use replay_rs::Player;

#[derive(Parser)]
#[command(name = "player")]
#[command(about = "Replay recorded terminal sessions with timing data")]
#[command(version = "0.1.0")]
struct Cli {
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Determine timing file name
    let timing_file = cli
        .timing
        .unwrap_or_else(|| format!("{}.timing", cli.session_file));

    if cli.verbose {
        println!("🎬 Session file: {}", cli.session_file);
        println!("⏱️  Timing file: {}", timing_file);
        if !cli.dump {
            println!("🚀 Speed: {}x", cli.speed);
        }
        println!(
            "📺 Mode: {}",
            if cli.dump {
                "Fast dump"
            } else {
                "Timed replay"
            }
        );
        println!();
    }

    // Create the player
    let player = Player::new(&timing_file, &cli.session_file)?;

    if cli.dump {
        // Fast dump mode
        if cli.verbose {
            println!("⚡ Fast dumping session content...");
            println!();
        }
        player.dump()?;
    } else {
        // Timed replay mode
        if cli.verbose {
            println!("🎭 Starting timed replay...");
            println!();
        }
        player.replay(cli.speed)?;
    }

    if cli.verbose {
        println!();
        println!("🎊 Playback completed!");
    }

    Ok(())
}
