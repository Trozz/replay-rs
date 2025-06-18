//! Simple example showing how to record and replay a command using replay-rs

use replay_rs::{Player, Recorder};
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_file = "example_session.log";
    let timing_file = "example_session.timing";

    println!("🎥 Recording a simple command...");

    // Record a command
    let recorder = Recorder::new(output_file, timing_file)?;
    let mut cmd = Command::new("echo");
    cmd.arg("Hello from replay-rs!");
    cmd.arg("This is a recorded session.");
    recorder.record_command(cmd, false)?;

    println!("\n✅ Recording complete!");

    // Wait a moment for dramatic effect
    std::thread::sleep(std::time::Duration::from_secs(1));

    println!("\n🎬 Now replaying the session...");
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Replay the session
    let player = Player::new(timing_file, output_file)?;
    player.replay(1.0)?;

    println!("🎉 Replay complete!");

    // Clean up files
    std::fs::remove_file(output_file).unwrap_or(());
    std::fs::remove_file(timing_file).unwrap_or(());

    Ok(())
}
