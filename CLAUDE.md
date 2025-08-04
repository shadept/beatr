# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Native Development
```bash
cargo run                    # Run native application in development mode
cargo build --release       # Build optimized native binary
cargo check                  # Fast compilation check without building
```

### WebAssembly Development
```bash
trunk serve                  # Serve web version locally (requires: cargo install trunk)
trunk build --release       # Build production web bundle
rustup target add wasm32-unknown-unknown  # Install wasm target (one-time setup)
```

### Prerequisites
- Rust 1.70+
- For web builds: `trunk` (install with `cargo install trunk`)
- Audio drivers (typically available by default on most systems)

## Architecture Overview

This is a real-time drum sequencer built with a clean separation between audio processing and UI:

### Core Architecture Pattern
The application uses **Arc<Mutex<T>>** for thread-safe communication between the audio thread and UI thread:
- `AudioEngine` owns the audio stream and manages `Sequencer` and `SampleBank` via shared pointers
- UI components interact with audio components through these shared references
- Audio processing happens in a separate callback thread via CPAL

### Key Components

**AudioEngine** (`src/audio/engine.rs`):
- Initializes CPAL audio stream with appropriate sample format handling
- Owns and coordinates `Sequencer` and `SampleBank` via `Arc<Mutex<T>>`
- Handles audio callback routing to sequencer processing

**Sequencer** (`src/audio/sequencer.rs`):
- Manages multiple drum patterns (16-step sequences)
- Handles voice allocation for sample playback
- Processes audio in real-time, triggering samples based on pattern state
- Controls timing via BPM and sample rate calculations

**SampleBank** (`src/audio/samples.rs`):
- Stores and manages drum samples (currently synthesized kick/snare/hihat)
- Handles sample loading from WAV files
- Provides sample generation via mathematical synthesis

**DrumComposerApp** (`src/ui/app.rs`):
- Main application state and egui integration
- Coordinates between UI components and audio engine
- Manages application lifecycle and error handling

### Threading Model
- **Main Thread**: UI rendering and user interaction via egui
- **Audio Thread**: Real-time audio processing via CPAL callback
- **Synchronization**: `Arc<Mutex<T>>` for shared state (sequencer patterns, sample bank)

### Audio Architecture - Dual Playback Modes
- **Timeline Mode**: Audio driven by timeline segments with specific patterns (Story 1.4)
- **Regular Mode**: Audio driven by sequencer's default patterns when timeline inactive
- Both modes use the same Arc<Mutex<T>> architecture for thread safety
- Audio callback intelligently switches between modes based on timeline playback state

### WebAssembly Compatibility
The codebase has dual compilation targets:
- Native builds use CPAL for audio I/O
- WASM builds include web-specific dependencies and entry points
- `#[cfg(target_arch = "wasm32")]` conditionally compiles web-specific code

## Key Files to Understand

- `src/main.rs`: Dual native/WASM entry points
- `src/audio/engine.rs`: CPAL audio setup and callback routing
- `src/audio/sequencer.rs`: Core sequencing logic and timing
- `src/ui/app.rs`: Main application state management
- `src/ui/components/pattern_grid.rs`: Step sequencer UI interaction

## Development Notes

- Audio processing must remain real-time safe (no allocations in audio callback)
- UI updates trigger via `ctx.request_repaint()` for smooth real-time visualization
- Pattern state changes go through mutex-protected sequencer methods
- Sample synthesis uses simple mathematical functions to avoid external audio file dependencies
- **IMPORTANT!** After each task, compile the native code and run all tests. Make sure the application compiles!

## Development Warnings

- Don't cargo run the app because it is UI based and you cannot properly interact with it. Make sure you have enough UI tests to validate functionality