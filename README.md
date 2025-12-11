# Rust Audio Visualizer

[![Rust Visualizer CI/CD](https://github.com/Baptiste-lg/Rust_visualizer/actions/workflows/Ci.yml/badge.svg)](https://github.com/Baptiste-lg/Rust_visualizer/actions/workflows/Ci.yml)
[![Docker Build & Push](https://github.com/Baptiste-lg/Rust_visualizer/actions/workflows/Docker.yml/badge.svg)](https://github.com/Baptiste-lg/Rust_visualizer/actions/workflows/Docker.yml)
[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://Baptiste-lg.github.io/Rust_visualizer/)

This project is a real-time audio visualizer built with Rust and the **Bevy** game engine. It transforms audio data from a file or a microphone into visual animations. With an interactive UI, you can switch between visualization modes and tweak parameters in real-time.

## Features

-   **Multiple Visualization Modes**: Choose from several visual scenes:
    -   **2D Bars**: A classic spectrum analyzer with vertical bars.
    -   **3D Cubes**: A 3D grid of cubes whose height and emissive light react to audio frequencies.
    -   **3D Orb**: A deformable sphere that ripples and pulses to the music using Perlin noise.
    -   **2D Disc**: A shader-based visualization that reacts to bass and rhythmic changes.
-   **Real-Time Audio Analysis**: Uses a Fast Fourier Transform (FFT) to break down the audio signal into different frequency bands.
-   **Flexible Audio Sources**: Load audio files (MP3, WAV) or use your microphone input.
-   **Intuitive Control Interface**: A user interface, built with `bevy_egui`, allows you to:
    -   Switch visualizers on the fly.
    -   Adjust parameters like sensitivity, colors, and visual effects.
    -   Control audio playback (play, pause, speed, seek).
-   **Interactive 3D Camera**: Explore the 3D scenes with pan-and-orbit camera controls.

## DevOps and CI/CD Pipeline

This project utilizes a robust CI/CD pipeline powered by **GitHub Actions**, demonstrating modern DevOps best practices:

-   **Automated Testing Matrix**: Builds and tests run concurrently on **Ubuntu, Windows, and macOS** to ensure cross-platform compatibility.
-   **Quality Gates**: Strict enforcement of `cargo fmt` and `cargo clippy`. The pipeline fails immediately if code standards are not met.
-   **Security Scanning**: Automated dependency auditing using `rust-audit` to detect vulnerabilities in the supply chain (DevSecOps).
-   **Smart Caching**: Implementation of `swatinem/rust-cache` to drastically reduce build times by caching Cargo registry and build artifacts.
-   **Documentation-as-Code**: Automatic generation and deployment of Rust documentation to **GitHub Pages** via OIDC authentication.
-   **Optimized Docker Builds**:
    -   **Multi-stage build** to keep the final image lightweight.
    -   Use of **Google Distroless** (Debian 12) base image for security (minimizing attack surface) and efficiency.
    -   Automatic publishing to **GitHub Container Registry (GHCR)** upon successful CI completion.

## How to Run

### Prerequisites

Before you begin, make sure you have **Rust** and **Cargo** installed. If not, follow the instructions at [rustup.rs](https://rustup.rs/).

You will also need to install the system dependencies required for **Bevy** and the audio libraries. Please see the [Bevy Environment Setup Guide](https://bevyengine.org/learn/book/getting-started/setup/) for instructions specific to your operating system (Linux, macOS, Windows).

### Launching the Project

1.  **Clone the repository**:
    ```bash
    git clone [https://github.com/Baptiste-lg/Rust_visualizer.git](https://github.com/Baptiste-lg/Rust_visualizer.git)
    cd Rust_visualizer
    ```

2.  **Compile and run the project** with Cargo:
    ```bash
    cargo run --release
    ```
    *The `--release` flag is recommended for optimal performance.*

### Using the Application

Once the application launches, you will be greeted by the main menu:

1.  **Main Menu**:
    -   Click **"Start Visualization"** to launch the last active visualizer.
    -   Click **"Select Microphone"** to choose an audio input device before starting.

2.  **Visualizer Interface**:
    -   **"Controls" Window**: Adjust global settings like the number of frequency bands, sensitivity, and options specific to each visualizer.
    -   **"Audio Source" Window**: Switch between microphone input and loading an audio file.
    -   **"Visualizers" Window**: Change the visualization mode.
    -   **"Playback Controls" Window** (if a file is loaded): Manage your music playback.