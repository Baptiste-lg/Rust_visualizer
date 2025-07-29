// This struct defines the uniform data passed from the Bevy application to the shader.
// It contains all the parameters needed to control the disc's appearance and animation.
struct DiscMaterial {
    color: vec4<f32>,
    time: f32,
    radius: f32,
    line_thickness: f32,
    iterations: f32,
    speed: f32,
    center_radius_factor: f32,
    resolution: vec2<f32>,
    bass: f32,
    flux: f32,
    zoom: f32,
};

// Bind the uniform data to a variable accessible within the shader.
@group(2) @binding(0)
var<uniform> material: DiscMaterial;

const PI : f32 = 3.1415926535;

// Draws a single circular arc with smoothed edges.
// The arc starts at angle 0 and ends at `angle_end`.
fn ring(p: vec2<f32>, radius: f32, thickness: f32, angle_end: f32) -> f32 {
    // atan2 returns an angle in the range [-PI, PI].
    var angle = atan2(p.y, p.x);
    let dist = length(p);

    // This is the key correction: map the angle from [-PI, PI] to [0, 2*PI].
    // This ensures the arc is drawn correctly without a seam at the -PI/PI boundary.
    if (angle < 0.0) {
        angle = angle + 2.0 * PI;
    }

    // Use a small value for smoothstep to create anti-aliased edges.
    let smooth_edge = 0.02;

    // Check if the pixel's angle falls within the desired arc range.
    // smoothstep creates a soft fade at the beginning and end of the arc.
    let angle_check = smoothstep(0.0, smooth_edge, angle) * smoothstep(angle_end, angle_end - smooth_edge, angle);

    // Check if the pixel's distance from the center falls within the ring's thickness.
    // This creates the ring shape itself.
    let radius_check = smoothstep(radius - thickness / 2.0, radius - thickness / 2.0 + smooth_edge, dist) -
                       smoothstep(radius + thickness / 2.0, radius + thickness / 2.0 + smooth_edge, dist);

    return radius_check * angle_check;
}


@fragment
fn fragment(
    @builtin(position) frag_coord: vec4<f32>
) -> @location(0) vec4<f32> {
    // --- Coordinate setup ---
    // Normalize the fragment's coordinates to a range of [-1, 1] and correct for aspect ratio.
    // This creates a UV space where (0,0) is the center of the screen.
    var p = (frag_coord.xy / material.resolution) * 2.0 - 1.0;
    p.x = p.x * (material.resolution.x / material.resolution.y);
    // Apply the camera zoom.
    p = p * material.zoom;

    // Make the radius and thickness react to the audio analysis.
    let reactive_radius = material.radius + (material.bass * 0.1);
    let reactive_thickness = material.line_thickness + (material.flux * 0.05);

    var final_frag: f32 = 0.0;
    // --- Main loop to draw the concentric rings ---
    for (var i = 0.0; i < material.iterations; i = i + 1.0) {
        let divi = i / material.iterations * material.center_radius_factor;
        let current_radius = reactive_radius - divi;

        // Calculate the end angle of the arc for this iteration.
        // The sine wave creates a smooth oscillation from 0.0 to 1.0.
        let sine_wave = (sin(material.time * material.speed - divi * 5.0) * -0.5 + 0.5);
        let full_circle = 2.0 * PI;
        // Add a small overcompensation to the angle to hide any potential seam.
        let overcompensation = 0.1;
        let end_angle = sine_wave * (full_circle + overcompensation);

        // Add the result of the ring drawing to the final fragment color.
        final_frag += ring(p, current_radius, reactive_thickness, end_angle);
    }

    // Combine the final shape with the material color and ensure it's not overly bright.
    let final_color = material.color.rgb * clamp(final_frag, 0.0, 1.0);
    return vec4<f32>(final_color, 1.0);
}