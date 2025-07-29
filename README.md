# Rust Audio Visualizer

This project is a real-time audio visualizer built with Rust and the **Bevy** game engine. It transforms audio data from a file or a microphone into captivating visual animations. With an interactive UI, you can switch between visualization modes and tweak parameters in real-time for a customized experience.

## Features

-   **Multiple Visualization Modes**: Choose from several visual scenes:
    -   **2D Bars**: A classic spectrum analyzer with vertical bars.
    -   **3D Cubes**: A 3D grid of cubes whose height and emissive light react to audio frequencies.
    -   **3D Orb**: A deformable sphere that ripples and pulses to the music using Perlin noise.
    -   **2D Disc**: A hypnotic, shader-based visualization that reacts to bass and rhythmic changes.
-   **Real-Time Audio Analysis**: Uses a Fast Fourier Transform (FFT) to break down the audio signal into different frequency bands.
-   **Flexible Audio Sources**: Load audio files (MP3, WAV) or use your microphone input.
-   **Intuitive Control Interface**: A user interface, built with `bevy_egui`, allows you to:
    -   Switch visualizers on the fly.
    -   Adjust parameters like sensitivity, colors, and visual effects.
    -   Control audio playback (play, pause, speed, seek).
-   **Interactive 3D Camera**: Explore the 3D scenes with easy-to-use pan-and-orbit camera controls.

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

Enjoy exploring your music like never before! ✨