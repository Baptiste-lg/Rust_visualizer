// src/viz_2d.rs

use bevy::prelude::*;
use crate::{AppState, audio::AudioAnalysis, VisualizationEnabled};

pub struct Viz2DPlugin;

impl Plugin for Viz2DPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Visualization2D), setup_2d_visuals)
            .add_systems(Update, update_2d_visuals
                .run_if(in_state(AppState::Visualization2D))
                .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0)
            );
    }
}

#[derive(Component)]
struct VizSquare;

fn setup_2d_visuals(mut commands: Commands) {
    info!("Setting up 2D visuals...");
    commands.spawn(Camera2dBundle::default());

    let square_size = 50.0;
    let padding = 10.0;
    let grid_size = 10;

    for x in 0..grid_size {
        for y in 0..grid_size {
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgb(0.2, 0.2, 0.8),
                        custom_size: Some(Vec2::new(square_size, square_size)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        (x - grid_size / 2) as f32 * (square_size + padding),
                        (y - grid_size / 2) as f32 * (square_size + padding),
                        0.0,
                    )),
                    ..default()
                },
                VizSquare,
            ));
        }
    }
}

// MODIFIED: This now uses the new frequency bins for visualization.
fn update_2d_visuals(
    audio_analysis: Res<AudioAnalysis>,
    mut query: Query<(&mut Sprite, &mut Transform), With<VizSquare>>,
) {
    if audio_analysis.frequency_bins.is_empty() {
        return;
    }

    // The first frequency bin (low bass) will control the color.
    let bass_amplitude = audio_analysis.frequency_bins[0];
    let bass_color = Color::rgb(
        0.2 + bass_amplitude * 0.8,
        0.2,
        0.8 - bass_amplitude * 0.4
    );

    // The overall treble average will control the scale of the squares.
    let treble_scale = 1.0 + audio_analysis.treble_average * 0.05;

    for (mut sprite, mut transform) in &mut query {
        sprite.color = bass_color;
        transform.scale = Vec3::splat(treble_scale);
    }
}