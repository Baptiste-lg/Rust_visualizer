// Le #import n'est plus nécessaire car on n'utilise plus `view`.

// La structure doit correspondre parfaitement à celle en Rust.
struct DiscMaterial {
    color: vec4<f32>,
    time: f32,
    radius: f32,
    line_thickness: f32,
    iterations: f32,
    speed: f32,
    center_radius_factor: f32,
    resolution: vec2<f32>, // Notre champ de résolution
};

// On lie notre matériau comme avant.
@group(2) @binding(0)
var<uniform> material: DiscMaterial;

// Fonction pour dessiner un arc.
fn ring(p: vec2<f32>, radius: f32, thickness: f32, angle_end: f32) -> f32 {
    let dist = length(p);
    let angle = atan2(p.y, p.x);

    let smooth_edge = 0.01;

    let angle_check = smoothstep(0.0, smooth_edge, angle) *
                      smoothstep(angle_end, angle_end - smooth_edge, angle);

    let radius_check = smoothstep(radius - thickness / 2.0, radius - thickness / 2.0 + smooth_edge, dist) -
                       smoothstep(radius + thickness / 2.0, radius + thickness / 2.0 + smooth_edge, dist);

    return radius_check * angle_check;
}

@fragment
fn fragment(
    // On prend la position du pixel comme donnée d'entrée.
    @builtin(position) frag_coord: vec4<f32>
) -> @location(0) vec4<f32> {

    // LA CORRECTION FINALE :
    // On normalise les coordonnées en utilisant notre propre variable `resolution`.
    // On n'a plus besoin de `view`.
    var p = (frag_coord.xy / material.resolution) * 2.0 - 1.0;
    p.x = p.x * (material.resolution.x / material.resolution.y);

    let ap = vec2<f32>(p.x, -p.y);
    var final_frag: f32 = 0.0;

    for (var i = 0.0; i < material.iterations; i = i + 1.0) {
        let divi = i / material.iterations * material.center_radius_factor;
        let current_radius = material.radius - divi;

        let end_angle = (sin(material.time * material.speed - divi * 5.0) * -1.5 + 1.5) * 3.1415926535;

        final_frag += ring(p, current_radius, material.line_thickness, end_angle);
        final_frag += ring(ap, current_radius, material.line_thickness, end_angle);
    }

    let final_color = material.color.rgb * clamp(final_frag, 0.0, 1.0);
    return vec4<f32>(final_color, 1.0);
}