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
    /// Session file to replay (defaults to session.log)
    #[arg(value_name = "SESSION_FILE", default_value = "session.log")]
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

    /// Input is in asciicast format (auto-detected if not specified)
    #[arg(short, long)]
    asciicast: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check if user explicitly provided timing file
    let explicit_timing = cli.timing.is_some();
    
    // Determine timing file name (empty if asciicast format)
    let timing_file = if cli.asciicast {
        String::new() // Not used for asciicast
    } else {
        cli.timing
            .unwrap_or_else(|| format!("{}.timing", cli.session_file))
    };

    // Auto-detect format if timing file doesn't exist but session file might be asciicast
    let auto_detect_asciicast = !cli.asciicast && 
        !explicit_timing && 
        !std::path::Path::new(&timing_file).exists();

    if cli.verbose {
        println!("🎬 Session file: {}", cli.session_file);
        if !cli.asciicast {
            println!("⏱️  Timing file: {}", timing_file);
        }
        if !cli.dump {
            println!("🚀 Speed: {}x", cli.speed);
        }
        println!(
            "📺 Mode: {}",
            if cli.dump {
                "Fast dump"
            } else if cli.asciicast {
                "Asciicast replay"
            } else {
                "Timed replay"
            }
        );
        println!();
    }

    // Create the player with auto-detection support
    let player = if auto_detect_asciicast {
        Player::new("", &cli.session_file)?
    } else {
        Player::new(&timing_file, &cli.session_file)?
    };

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
        if cli.asciicast || auto_detect_asciicast {
            player.replay_format(cli.speed, true)?;
        } else {
            player.replay_format(cli.speed, false)?;
        }
    }

    if cli.verbose {
        println!();
        println!("🎊 Playback completed!");
    }

    Ok(())
}
