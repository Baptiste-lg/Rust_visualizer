use bevy::prelude::*;
use rodio::{Decoder, OutputStream, Sink}; // Import rodio components
use std::fs::File;
use std::io::BufReader;

// This resource will hold the audio output stream, keeping it alive.
#[derive(Resource)]
struct AudioStream(OutputStream);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup, setup_audio_playback))
        .run();
}

/// Sets up a basic 3D scene with a camera and a light source.
fn setup(mut commands: Commands) {
    // Spawn a 3D camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Spawn a point light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}

/// Loads and plays a hardcoded audio file using rodio.
fn setup_audio_playback(mut commands: Commands) {
    // Get an output stream handle to the default physical sound device
    let (stream, stream_handle) = OutputStream::try_default().unwrap();

    // Load a sound from a file, using a buffered reader.
    let file = BufReader::new(File::open("Intro Gotaga !.mp3").unwrap());

    // Decode that sound file into a source
    let source = Decoder::new(file).unwrap();

    // Create a Sink to play the sound
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.append(source);
    sink.play();

    // The sink is detached here, meaning it will play until it is done.
    // We must store the output stream to keep it alive.
    sink.detach();
    commands.insert_resource(AudioStream(stream));

    // Log to console to confirm it's working
    info!("Successfully started playing 'Intro Gotaga !.mp3'");
}