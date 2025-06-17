# replay-rs

A Rust library for recording and replaying terminal sessions with timing data, compatible with the classic Unix `script` and `scriptreplay` tools but implemented entirely in Rust with cross-platform support.

[![Crates.io](https://img.shields.io/crates/v/replay-rs.svg)](https://crates.io/crates/replay-rs)
[![Documentation](https://docs.rs/replay-rs/badge.svg)](https://docs.rs/replay-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **🎥 Record terminal sessions**: Capture command output with precise timing data
- **🎬 Replay with speed control**: Play back sessions at different speeds (like asciinema)
- **🎨 ANSI sequence handling**: Clean up problematic control sequences while preserving colors
- **🖥️ Cross-platform**: Works on macOS, Linux, and other Unix-like systems
- **⚡ Zero external dependencies**: Built-in implementation, no need for external tools
- **📄 Multiple formats**: Support for both raw binary and cleaned text output
- **🔧 Compatible**: Works with existing `script`/`scriptreplay` timing files

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
replay-rs = "0.1"
```

### Basic Usage

```rust
use replay_rs::{Recorder, Player};
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Record a command
    let recorder = Recorder::new("session.log", "session.log.timing")?;
    let mut cmd = Command::new("echo");
    cmd.arg("Hello, World!");
    recorder.record_command(cmd, false)?; // false = binary format, true = text format

    // Replay the session
    let player = Player::new("session.log.timing", "session.log")?;
    player.replay(1.0)?; // 1.0 = normal speed, 2.0 = 2x speed, etc.
    
    Ok(())
}
```

### Advanced Usage

```rust
use replay_rs::{Recorder, Player, clean_for_display};
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Record with text cleaning (removes problematic ANSI sequences)
    let recorder = Recorder::new("clean_session.log", "clean_session.timing")?;
    let mut cmd = Command::new("ls");
    cmd.arg("--color=always");
    recorder.record_command(cmd, true)?; // true = clean text format
    
    // Replay at different speeds
    let player = Player::new("clean_session.timing", "clean_session.log")?;
    
    println!("Playing at 2x speed:");
    player.replay(2.0)?;
    
    println!("Dumping without timing:");
    player.dump()?; // Fast dump without delays
    
    // Manual ANSI cleaning
    let raw_output = "?2004h\x1b[32mGreen text\x1b[0m?2004l";
    let cleaned = clean_for_display(raw_output);
    println!("Cleaned: {}", cleaned); // Outputs: "Green text" (with color preserved)
    
    Ok(())
}
```

## API Documentation

### Recorder

The `Recorder` struct captures command execution with timing data.

```rust
// Create a new recorder
let recorder = Recorder::new("output.log", "timing.log")?;

// Record a command (plain_text: false = raw, true = cleaned)
recorder.record_command(command, plain_text)?;
```

### Player

The `Player` struct replays recorded sessions.

```rust
// Create a player
let player = Player::new("timing.log", "output.log")?;

// Replay with speed control
player.replay(speed_multiplier)?; // 1.0 = normal, 2.0 = 2x, 0.5 = half

// Fast dump without timing
player.dump()?;
```

### Utility Functions

```rust
// Clean ANSI sequences while preserving colors
let cleaned = clean_for_display(raw_text);
```

## File Format Compatibility

replay-rs uses the same timing file format as the classic Unix `scriptreplay` command:

```
delay_in_seconds byte_count
delay_in_seconds byte_count
...
```

This means you can:
- Use replay-rs to play files recorded with `script -t`
- Use `scriptreplay` to play files recorded with replay-rs
- Mix and match tools as needed

## Use Cases

- **📚 Documentation**: Record setup procedures and tutorials
- **🎓 Training**: Create step-by-step command demonstrations  
- **🐛 Debugging**: Capture and share terminal sessions for troubleshooting
- **📊 Automation**: Record command outputs for later analysis
- **🔍 Auditing**: Maintain logs of terminal activities
- **🎮 Demos**: Create smooth terminal recordings for presentations

## Comparison with Other Tools

| Feature | replay-rs | asciinema | script/scriptreplay | ttyrec |
|---------|-----------|-----------|-------------------|---------|
| Cross-platform | ✅ | ✅ | ⚠️ (Unix only) | ⚠️ (Unix only) |
| Speed control | ✅ | ✅ | ✅ | ❌ |
| No external deps | ✅ | ❌ | ⚠️ (system tools) | ❌ |
| ANSI cleaning | ✅ | ❌ | ❌ | ❌ |
| Library API | ✅ | ❌ | ❌ | ❌ |
| File compatibility | ✅ | ❌ | ✅ | ❌ |

## Examples

Run the included examples:

```bash
# Simple recording and playback
cargo run --example simple_record_replay

# More examples coming soon!
```

## Real-World Usage

This library was extracted from the [aws-ssm-connector](https://github.com/trozz/aws-ssm-connector) project, where it's used to record and replay AWS SSM sessions with perfect timing and color preservation.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by the classic Unix `script` and `scriptreplay` utilities
- Built with modern Rust for safety and cross-platform compatibility
- Thanks to the asciinema project for pioneering terminal session recording