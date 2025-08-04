# Beatr - Drum Track Composer

A drum track/loop composer built with Rust and egui, featuring native performance with WebAssembly portability.

## Features

- Real-time drum sequencing with 16-step patterns
- Built-in synthesized drum samples (kick, snare, hi-hat)
- Adjustable tempo (60-200 BPM)
- Transport controls (play, pause, stop)
- Pattern grid interface for step programming
- Cross-platform native application
- WebAssembly support for web deployment

## Building and Running

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- Audio drivers (should be automatically available on most systems)

### Native Application

```bash
# Clone the repository
git clone <repository-url>
cd beatr

# Run in development mode
cargo run

# Build release binary
cargo build --release
```

### Web Version (WebAssembly)

```bash
# Install trunk for web building
cargo install trunk

# Install wasm target
rustup target add wasm32-unknown-unknown

# Serve locally (development)
trunk serve

# Build for production
trunk build --release
```

## Project Structure

```
src/
├── main.rs              # Application entry point
├── audio/               # Audio engine and processing
│   ├── mod.rs
│   ├── engine.rs        # Core audio engine with CPAL
│   ├── samples.rs       # Sample management and synthesis
│   └── sequencer.rs     # Pattern sequencing logic
└── ui/                  # User interface components
    ├── mod.rs
    ├── app.rs           # Main application state
    └── components/      # UI widgets
        ├── mod.rs
        ├── pattern_grid.rs  # Step sequencer grid
        ├── transport.rs     # Play/pause/stop controls
        └── tempo.rs         # BPM control
```

## Usage

1. **Transport Controls**: Use play/pause/stop buttons to control playback
2. **Tempo**: Adjust BPM using the slider or preset buttons (80, 120, 140, 160)
3. **Pattern Programming**: Click the circular step buttons to enable/disable drum hits
4. **Pattern Management**: Use "Clear" buttons to reset individual patterns

## Architecture

- **Audio Engine**: Uses CPAL for cross-platform audio I/O
- **Synthesis**: Built-in drum sample generation using mathematical synthesis
- **Sequencer**: Multi-pattern step sequencer with voice management
- **UI**: Immediate mode GUI with egui for responsive real-time updates
- **WebAssembly**: Full compatibility for web deployment without plugins

## Dependencies

### Core Audio
- `cpal` - Cross-platform audio I/O
- `fundsp` - Digital signal processing
- `hound` - WAV file support

### GUI
- `eframe` - egui application framework
- `egui_extras` - Additional egui widgets

### Utilities
- `serde` - Serialization for settings
- `anyhow` - Error handling

## License

MIT OR Apache-2.0