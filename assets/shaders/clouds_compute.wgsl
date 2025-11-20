#import bevy_open_world::common

const WORLEY_RESOLUTION = 32;
const WORLEY_RESOLUTION_F32 = 32.0;

struct Config {
    march_steps: u32,
    self_shadow_steps: u32,
    earth_radius: f32,
    bottom: f32,
    top: f32,
    coverage: f32,
    detail_strength: f32,
    base_edge_softness: f32,
    bottom_softness: f32,
    density: f32,
    shadow_march_step_size: f32,
    shadow_march_step_multiply: f32,
    forward_scattering_g: f32,
    backward_scattering_g: f32,
    scattering_lerp: f32,
    ambient_color_top: vec4f,
    ambient_color_bottom: vec4f,
    min_transmittance: f32,
    base_scale: f32,
    detail_scale: f32,
    sun_dir: vec4f,
    sun_color: vec4f,
    camera_translation: vec3f,
    debug: f32,
    time: f32,
    reprojection_strength: f32,
    render_resolution: vec2f,
    inverse_camera_view: mat4x4f,
    inverse_camera_projection: mat4x4f,
    wind_displacement: vec3f,
};

@group(0) @binding(0) var<uniform> config: Config;

@group(1) @binding(0) var clouds_render_texture: texture_storage_2d<rgba32float, read_write>;
@group(1) @binding(1) var clouds_atlas_texture: texture_storage_2d<rgba32float, read_write>;
@group(1) @binding(2) var clouds_worley_texture: texture_storage_3d<rgba32float, read_write>;
@group(1) @binding(3) var sky_texture: texture_storage_2d<rgba32float, read_write>;

struct RaymarchResult {
    dist: f32,
    color: vec4f,
}

fn henyey_greenstein(ray_dot_sun: f32, g: f32) -> f32 {
    let g_squared = g * g;
    return (1.0 - g_squared) / pow(1.0 + g_squared - 2.0 * g * ray_dot_sun, 1.5);
}

fn intersect_earth_sphere(ray_dir: vec3f, radius: f32) -> f32 {
    let bottom = config.earth_radius * ray_dir.y;
    let d = bottom * bottom + radius * radius + 2.0 * config.earth_radius * radius;
    return sqrt(d) - bottom;
}

fn cloud_map_base(p: vec3f, normalized_height: f32) -> f32 {
	let uv = abs(p * (0.00005 * config.base_scale) * config.render_resolution.xyy);
    let cloud = textureLoad(
        clouds_atlas_texture,
         vec2u(
            u32(uv.x) % u32(config.render_resolution.x),
            u32(uv.z) % u32(config.render_resolution.y)
        )
    ).rgb;

    var n = normalized_height * normalized_height * cloud.b + pow(1.0 - normalized_height, 16.0);
	return common::remap(cloud.r - n, cloud.g, 1.0);
}

fn cloud_map_detail(position: vec3f) -> f32 {
    let p = abs(position) * (0.0016 * config.base_scale * config.detail_scale);

    // TODO: add bilinear filtering
    var p1 = p % 32.0;
    let a = textureLoad(clouds_worley_texture, vec3u(u32(p1.x), u32(p1.y), u32(p1.z))).r;

    // TODO: add bilinear filtering
    let p2 = (p + 1.0) % 32.0;
    let b = textureLoad(clouds_worley_texture, vec3u(u32(p2.x), u32(p2.y), u32(p2.z))).r;

    return mix(a, b, fract(p.y));
}

// Erode a bit from the bottom and top of the cloud layer
fn cloud_gradient(normalized_height: f32) -> f32 {
    return (
        common::linearstep(0.0, 0.1, normalized_height) -
        common::linearstep(0.8, 1.2, normalized_height)
    );
}

fn cloud_map(pos: vec3f, normalized_height: f32) -> f32 {
    let ps = pos;

    var m = cloud_map_base(ps, normalized_height) * cloud_gradient(normalized_height);

	let detail_strength = smoothstep(1.0, 0.5, m);

    // Erode with detail
    if detail_strength > 0.0 {
		m -= cloud_map_detail(ps) * detail_strength * config.detail_strength;
    }

	m = smoothstep(0.0, config.base_edge_softness, m + config.coverage - 1.0);
    m *= common::linearstep0(config.bottom_softness, normalized_height);

    return clamp(m * config.density * (1.0 + max((ps.x - 7000.0) * 0.005, 0.0)), 0.0, 1.0);
}

fn volumetric_shadow(origin: vec3f, ray_dot_sun: f32) -> f32{
    var ray_step_size = config.shadow_march_step_size;
    var distance_along_ray = ray_step_size * 0.5;
    var shadow = 1.0;
    let clouds_height = config.top - config.bottom;

    for (var s: u32 = 0; s < config.self_shadow_steps; s++) {
        let pos = origin + config.sun_dir.xyz * distance_along_ray;
        let normalized_height = (length(pos) - (config.earth_radius + config.bottom)) / clouds_height;

        if (normalized_height > 1.0) { return shadow; };

        let density = cloud_map(pos, normalized_height);
        shadow *= exp(-density * ray_step_size);

        ray_step_size *= config.shadow_march_step_multiply;
        distance_along_ray += ray_step_size;
    }

    return shadow;
}

fn raymarch(_ray_origin: vec3f, ray_dir: vec3f, _dist: f32) -> RaymarchResult {
    var dist = _dist;

    if (ray_dir.y < 0.0) {
        return RaymarchResult(dist, vec4f(0.0, 0.0, 0.0, 10.0));
    }

    let ro_xz = _ray_origin.xz;

    let ray_origin = vec3f(
        ro_xz.x,
        sqrt(config.earth_radius * config.earth_radius - dot(ro_xz, ro_xz)),
        ro_xz.y
    );

    let start = intersect_earth_sphere(ray_dir, config.bottom);
    var end = intersect_earth_sphere(ray_dir, config.top);

    if (start > dist) {
        return RaymarchResult(dist, vec4f(0.0, 0.0, 0.0, 10.0));
    }

    end = min(end, dist);

    let ray_dot_sun = dot(ray_dir, -config.sun_dir.xyz);

    let step_distance = (end - start) / f32(config.march_steps);
    let hashed_offset = common::hash13(ray_dir + fract(config.time));
    var dir_length = start - step_distance * hashed_offset;

    // Frostbite: dual-lobe phase function
    let scattering = mix(
        henyey_greenstein(ray_dot_sun, config.forward_scattering_g),
        henyey_greenstein(ray_dot_sun, config.backward_scattering_g),
        config.scattering_lerp
    );

    var transmittance = 1.0;
    var scattered_light = vec3f(0.0, 0.0, 0.0);

    dist = config.earth_radius;
    let clouds_height = config.top - config.bottom;

    for (var s: u32 = 0; s < config.march_steps; s++) {
        let p = ray_origin + dir_length * ray_dir;

        let normalized_height = clamp(
            (length(p) - (config.earth_radius + config.bottom)) / clouds_height,
            0.0,
            1.0
        );

        let density_sampled = cloud_map(p, normalized_height);

        if (density_sampled > 0.0) {
            dist = min(dist, dir_length);
            let ambient_light = mix(config.ambient_color_bottom, config.ambient_color_top, normalized_height).rgb;

            // Frostbite energy-conversing integration
            let S = (ambient_light + config.sun_color.rgb * (scattering * volumetric_shadow(p, ray_dot_sun))) * density_sampled;
            let delta_transmittance = exp(-density_sampled * step_distance);
            let integrated_scattering = (S - S * delta_transmittance) / density_sampled;

            scattered_light += transmittance * integrated_scattering;
            transmittance *= delta_transmittance;
        }

        if transmittance <= config.min_transmittance { break; }

        dir_length += step_distance;
    }

    return RaymarchResult(dist, vec4f(scattered_light, transmittance));
}

// Fast skycolor function by Íñigo Quílez
// https://www.shadertoy.com/view/MdX3Rr
fn get_sky_color(ray_dir: vec3f) -> vec3f {
    let sundot = clamp(dot(ray_dir,config.sun_dir.xyz),0.0,1.0);
	var col = vec3f(0.2,0.5,0.85)*1.1 - max(ray_dir.y,0.01)*max(ray_dir.y,0.01)*0.5;
    col = mix(col, 0.85*vec3(0.7,0.75,0.85), pow(1.0-max(ray_dir.y,0.0), 6.0) );

    col += 0.25*vec3f(1.0,0.7,0.4)*pow( sundot,5.0 );
    col += 0.25*vec3f(1.0,0.8,0.6)*pow( sundot,64.0 );
    col += 0.20*vec3f(1.0,0.8,0.6)*pow( sundot,512.0 );

    col += clamp((0.1-ray_dir.y)*10., 0., 1.) * vec3f(.0,.1,.2);
    col += 0.2*vec3f(1.0,0.8,0.6)*pow( sundot, 8.0 );
    return col;
}

fn render_clouds_atlas(frag_coord: vec2f) -> vec4f {
    let v_uv = frag_coord / config.render_resolution.xy;
    let coord = vec3f(v_uv, 0.5);

    let mfbm = 0.9;
    let mvor = 0.7;

    return vec4f(
        mix(
            1.0,
            common::tilable_perlin_fbm(coord, 7, 4),
            mfbm
        ) * mix(
            1.0,
            common::tilable_voronoi(coord, 8, 9.0),
            mvor
        ),
        0.625 * common::tilable_voronoi(coord, 3, 15.0) +
            0.250 * common::tilable_voronoi(coord, 3, 19.0) +
            0.125 * common::tilable_voronoi(coord, 3, 23.0) -
            1.0,
        1.0 - common::tilable_voronoi(coord + 0.5, 6, 9.0),
        1.0
    );
}

fn render_clouds_worley(coord: vec3f) -> vec4f {
    let r = common::tilable_voronoi(coord, 16, 3.0);
    let g = common::tilable_voronoi(coord, 4, 8.0);
    let b = common::tilable_voronoi(coord, 4, 16.0);

    let c = max(0.0, 1.0 - (r + g * 0.5 + b * 0.25) / 1.75);

    return vec4f(c);
}

fn main_image(frag_coord: vec2f, camera: mat4x4f, old_cam: mat4x4f, ray_dir: vec3f, ray_origin: vec3f) -> vec4f {
    if (frag_coord.y < 1.0) {
        if frag_coord.x < 1.0 { return vec4f(config.render_resolution.xy, 0.0, 0.0); }
        return common::save_camera(camera, frag_coord, ray_origin);
    }

    var dist = 1e9;
    var col = vec4f(0.0, 0.0, 0.0, 1.0);

    if ray_dir.y > 0.0 {
        let result = raymarch(ray_origin, ray_dir, dist);
        col = result.color;
        dist = result.dist;

        let fog = 1.0 - (0.1 + exp(-dist * 0.0001));
        col = vec4f(mix(col.rgb, get_sky_color(ray_dir) * (1.0 - col.a), fog), col.a);
    }

    if col.w > 1.0 {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }

    let old_cam_col = textureLoad(clouds_render_texture, vec2u(1, u32(config.render_resolution.y) - 1));
    let new_cam_col = camera[0];

    if abs(old_cam_col[0] - new_cam_col[0]) > 0.0001 {
        return col;
    }

    let original_color = textureLoad(
        clouds_render_texture,
        vec2u(u32(frag_coord.x),
        u32(config.render_resolution.y - 1.0) - u32(frag_coord.y))
    );
    return mix(col, original_color, config.reprojection_strength);
}

fn get_ray_origin(time: f32) -> vec3f {
    return config.camera_translation - config.wind_displacement;
}

fn get_ray_direction(frag_coord: vec2f) -> vec3f {
    // inverse_camera_projection is also called view_from_clip
    // inverse_camera_view is also called world_from_view
    let rect_relative = frag_coord / config.render_resolution;

    // Flip the Y co-ordinate from the top to the bottom to enter NDC.
    let ndc_xy = (rect_relative * 2.0 - vec2f(1.0, 1.0)) * vec2f(1.0, -1.0);

    let ray_clip = vec4f(ndc_xy.xy, -1.0, 1.0);
    let ray_eye = config.inverse_camera_projection * ray_clip;
    let ray_world = config.inverse_camera_view * vec4f(ray_eye.xy, -1.0, 0.0);

    return normalize(ray_world.xyz);
}

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let index = vec2f(f32(invocation_id.x), f32(invocation_id.y));
    let inverted_y_coord = config.render_resolution.y - index.y;

    let worley_coord = vec2f(0.5) + vec2f(index.x, inverted_y_coord);

    let z = floor(worley_coord.x / WORLEY_RESOLUTION_F32) + 8.0 * floor(worley_coord.y / WORLEY_RESOLUTION_F32);
    let xy = vec2f(index.x, inverted_y_coord) % WORLEY_RESOLUTION_F32;
    let xyz = vec3f(xy, z);

    let worley_col = render_clouds_worley(xyz / WORLEY_RESOLUTION_F32);
    let atlas_col = render_clouds_atlas(vec2f(index.x, inverted_y_coord));

    storageBarrier();

    textureStore(clouds_atlas_texture, invocation_id.xy, atlas_col);
    textureStore(clouds_worley_texture, vec3u(u32(xyz.x), u32(xyz.y), u32(xyz.z)), worley_col);
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let index = vec2f(f32(invocation_id.x), f32(invocation_id.y));

    // Load old camera matrix before storageBarrier to prevent race conditions;
    let old_cam = common::load_camera(clouds_render_texture);
    var frag_coord = vec2f(index.x + 0.5, config.render_resolution.y - 0.5 - index.y);

    var ray_origin = get_ray_origin(config.time);
    var ray_dir = get_ray_direction(vec2f(index.x + 0.5, 0.5 + index.y));
    var col = main_image(frag_coord, config.inverse_camera_view, old_cam, ray_dir, ray_origin);

    storageBarrier();

    textureStore(clouds_render_texture, invocation_id.xy, col);
    textureStore(sky_texture, invocation_id.xy, vec4f(get_sky_color(ray_dir), 1.0));
}
