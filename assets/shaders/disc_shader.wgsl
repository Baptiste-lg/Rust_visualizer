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
    padding: f32,
};

@group(2) @binding(0)
var<uniform> material: DiscMaterial;

const PI : f32 = 3.1415926535;

fn ring(p: vec2<f32>, radius: f32, thickness: f32, angle_end: f32) -> f32 {
    var angle = atan2(p.y, p.x);
    let dist = length(p);

    if (angle < 0.0) {
        angle = angle + 2.0 * PI;
    }

    let smooth_edge = 0.02;

    let angle_check = smoothstep(0.0, smooth_edge, angle) * smoothstep(angle_end, angle_end - smooth_edge, angle);

    let radius_check = smoothstep(radius - thickness / 2.0, radius - thickness / 2.0 + smooth_edge, dist) -
                       smoothstep(radius + thickness / 2.0, radius + thickness / 2.0 + smooth_edge, dist);

    return radius_check * angle_check;
}

@fragment
fn fragment(
    @builtin(position) frag_coord: vec4<f32>
) -> @location(0) vec4<f32> {
    var p = (frag_coord.xy / material.resolution) * 2.0 - 1.0;
    p.x = p.x * (material.resolution.x / material.resolution.y);
    p = p * material.zoom;

    let reactive_radius = material.radius + (material.bass * 0.1);
    let reactive_thickness = material.line_thickness + (material.flux * 0.05);

    var final_frag: f32 = 0.0;
    for (var i = 0.0; i < material.iterations; i = i + 1.0) {
        let divi = i / material.iterations * material.center_radius_factor;
        let current_radius = reactive_radius - divi;

        let sine_wave = (sin(material.time * material.speed - divi * 5.0) * -0.5 + 0.5);
        let full_circle = 2.0 * PI;
        let overcompensation = 0.1;
        let end_angle = sine_wave * (full_circle + overcompensation);

        final_frag += ring(p, current_radius, reactive_thickness, end_angle);
    }

    let final_color = material.color.rgb * clamp(final_frag, 0.0, 1.0);
    return vec4<f32>(final_color, 1.0);
}