// This struct defines the data that gets passed from the main application to the shader.
// These are "uniform" variables, meaning they are the same for every pixel being rendered in a single draw call.
struct DiscMaterial {
    // The base color of the visualizer rings.
    color: vec4<f32>,
    // A timer value, usually the elapsed time since the start of the application. Used for animation.
    time: f32,
    // The base radius of the rings.
    radius: f32,
    // The base thickness of the lines that make up the rings.
    line_thickness: f32,
    // The number of concentric rings to draw.
    iterations: f32,
    // The speed of the animation.
    speed: f32,
    // A factor that controls how much the radius of the inner rings shrinks.
    center_radius_factor: f32,
    // The resolution (width and height) of the screen or window.
    resolution: vec2<f32>,
    // The analyzed bass level from the audio.
    bass: f32,
    // The analyzed spectral flux from the audio (how much the sound is changing).
    flux: f32,
    // The camera's zoom level, used to scale the entire visual.
    zoom: f32,
};

// Binds the `DiscMaterial` struct to a specific location in the shader's memory layout.
@group(2) @binding(0)
var<uniform> material: DiscMaterial;

// A constant for the value of Pi.
const PI : f32 = 3.1415926535;

// This function calculates the brightness of a single, partially drawn ring.
// It returns a value between 0.0 (not part of the ring) and 1.0 (fully part of the ring).
fn ring(p: vec2<f32>, radius: f32, thickness: f32, angle_end: f32) -> f32 {
    // Calculate the angle and distance of the current pixel from the center (0,0).
    var angle = atan2(p.y, p.x);
    let dist = length(p);

    // Map the angle from [-PI, PI] to [0, 2*PI] for easier calculations.
    if (angle < 0.0) {
        angle = angle + 2.0 * PI;
    }

    // `smoothstep` creates a soft edge for the shapes, avoiding hard, aliased lines.
    let smooth_edge = 0.02;

    // Check if the pixel's angle is within the drawn portion of the ring.
    // This creates the "wiping" or "growing" arc effect.
    let angle_check = smoothstep(0.0, smooth_edge, angle) * smoothstep(angle_end, angle_end - smooth_edge, angle);

    // Check if the pixel's distance from the center falls within the ring's thickness.
    let radius_check = smoothstep(radius - thickness / 2.0, radius - thickness / 2.0 + smooth_edge, dist) -
                       smoothstep(radius + thickness / 2.0, radius + thickness / 2.0 + smooth_edge, dist);

    // The final value is only non-zero if the pixel is within both the correct angle and the correct radius.
    return radius_check * angle_check;
}

// This is the main function of the fragment shader. It runs for every single pixel
// on the screen and determines its final color.
@fragment
fn fragment(
    @builtin(position) frag_coord: vec4<f32>
) -> @location(0) vec4<f32> {
    // --- Coordinate Setup ---
    // Normalize the pixel coordinates from [0, resolution] to [-1, 1], with (0,0) at the center.
    var p = (frag_coord.xy / material.resolution) * 2.0 - 1.0;
    // Correct for the aspect ratio of the screen to prevent stretching.
    p.x = p.x * (material.resolution.x / material.resolution.y);
    // Apply the camera zoom.
    p = p * material.zoom;


    // --- Reactivity ---
    // Make the radius and thickness change based on the audio analysis.
    let reactive_radius = material.radius + (material.bass * 0.1);
    let reactive_thickness = material.line_thickness + (material.flux * 0.05);

    // --- Ring Drawing Loop ---
    // Start with a black fragment and add light from each ring.
    var final_frag: f32 = 0.0;
    for (var i = 0.0; i < material.iterations; i = i + 1.0) {
        // Calculate the radius for the current ring, making inner rings smaller.
        let divi = i / material.iterations * material.center_radius_factor;
        let current_radius = reactive_radius - divi;

        // Create a sine wave that oscillates between 0.0 and 1.0, driven by time.
        // This will control the angle of the arc.
        let sine_wave = (sin(material.time * material.speed - divi * 5.0) * -0.5 + 0.5);
        let full_circle = 2.0 * PI;
        let overcompensation = 0.1; // A small value to ensure the circle closes smoothly.
        let end_angle = sine_wave * (full_circle + overcompensation);

        // Add the brightness value of the current ring to the total.
        final_frag += ring(p, current_radius, reactive_thickness, end_angle);
    }

    // --- Final Color ---
    // Multiply the base material color by the calculated brightness.
    // `clamp` ensures the value stays between 0.0 and 1.0.
    let final_color = material.color.rgb * clamp(final_frag, 0.0, 1.0);
    // Return the final color with full alpha (non-transparent).
    return vec4<f32>(final_color, 1.0);
}