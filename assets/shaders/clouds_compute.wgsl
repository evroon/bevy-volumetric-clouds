#import bevy_open_world::common

const EPSILON = 0.000001;
const WORLEY_RESOLUTION = 32;
const WORLEY_RESOLUTION_F32 = 32.0;

struct Config {
    clouds_base_scale: f32,
    clouds_raymarch_steps_count: u32,
    clouds_bottom_height: f32,
    clouds_top_height: f32,
    clouds_coverage: f32,
    clouds_density: f32,
    clouds_detail_scale: f32,
    clouds_detail_strength: f32,
    clouds_base_edge_softness: f32,
    clouds_bottom_softness: f32,
    clouds_shadow_raymarch_steps_count: u32,
    clouds_shadow_raymarch_step_size: f32,
    clouds_shadow_raymarch_step_multiply: f32,
    clouds_ambient_color_top: vec4f,
    clouds_ambient_color_bottom: vec4f,
    clouds_min_transmittance: f32,
    planet_radius: f32,
    forward_scattering_g: f32,
    backward_scattering_g: f32,
    scattering_lerp: f32,
    sun_dir: vec4f,
    sun_color: vec4f,
    camera_translation: vec3f,
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

fn cloud_map_base(p: vec3f, normalized_height: f32) -> f32 {
	let uv = abs(p * (0.00005 * config.clouds_base_scale) * config.render_resolution.xyy);
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
    let p = abs(position) * (0.0016 * config.clouds_base_scale * config.clouds_detail_scale);

    // TODO: add bilinear filtering
    var p1 = p % 32.0;
    let a = textureLoad(clouds_worley_texture, vec3u(u32(p1.x), u32(p1.y), u32(p1.z))).r;

    // TODO: add bilinear filtering
    let p2 = (p + 1.0) % 32.0;
    let b = textureLoad(clouds_worley_texture, vec3u(u32(p2.x), u32(p2.y), u32(p2.z))).r;

    return mix(a, b, fract(p.y));
}

// Erode a bit from the clouds_bottom_height and clouds_top_height of the cloud layer
fn cloud_gradient(normalized_height: f32) -> f32 {
    return (
        common::linearstep(0.0, 0.1, normalized_height) -
        common::linearstep(0.8, 1.2, normalized_height)
    );
}

fn get_cloud_map_density(pos: vec3f, normalized_height: f32) -> f32 {
    let ps = pos;

    var m = cloud_map_base(ps, normalized_height) * cloud_gradient(normalized_height);

	let clouds_detail_strength = smoothstep(1.0, 0.5, m);

    // Erode with detail
    if clouds_detail_strength > 0.0 {
		m -= cloud_map_detail(ps) * clouds_detail_strength * config.clouds_detail_strength;
    }

	m = smoothstep(0.0, config.clouds_base_edge_softness, m + config.clouds_coverage - 1.0);
    m *= common::linearstep0(config.clouds_bottom_softness, normalized_height);

    return clamp(m * config.clouds_density * (1.0 + max((ps.x - 7000.0) * 0.005, 0.0)), 0.0, 1.0);
}

fn get_normalized_height(pos: vec3f) -> f32 {
    let clouds_height = config.clouds_top_height - config.clouds_bottom_height;
    return (length(pos) - (config.planet_radius + config.clouds_bottom_height)) / clouds_height;
}

fn volumetric_shadow(origin: vec3f, ray_dot_sun: f32) -> f32 {
    var ray_step_size = config.clouds_shadow_raymarch_step_size;
    var distance_along_ray = ray_step_size * 0.5;
    var transmittance = 1.0;

    for (var step: u32 = 0; step < config.clouds_shadow_raymarch_steps_count; step++) {
        let pos = origin + config.sun_dir.xyz * distance_along_ray;
        let normalized_height = get_normalized_height(pos);

        if (normalized_height > 1.0) { return transmittance; };

        let clouds_density = get_cloud_map_density(pos, normalized_height);
        transmittance *= exp(-clouds_density * ray_step_size);

        ray_step_size *= config.clouds_shadow_raymarch_step_multiply;
        distance_along_ray += ray_step_size;
    }

    return transmittance;
}

fn intersect_planet_sphere(ray_dir: vec3f, sample_radius: f32) -> f32 {
    let clouds_bottom_height = config.planet_radius * ray_dir.y;
    let d = (
        clouds_bottom_height * clouds_bottom_height +
        sample_radius * sample_radius +
        2.0 * config.planet_radius * sample_radius
    );
    return sqrt(d) - clouds_bottom_height;
}

fn henyey_greenstein(ray_dot_sun: f32, g: f32) -> f32 {
    let g_squared = g * g;
    return (1.0 - g_squared) / pow(1.0 + g_squared - 2.0 * g * ray_dot_sun, 1.5);
}

fn raymarch(_ray_origin: vec3f, ray_dir: vec3f, max_dist: f32) -> RaymarchResult {
    if (ray_dir.y < 0.0) {
        return RaymarchResult(max_dist, vec4f(0.0, 0.0, 0.0, 1.0));
    }

    let ray_origin = vec3f(
        _ray_origin.x,
        config.planet_radius - _ray_origin.y,
        _ray_origin.z
    );

    let start = intersect_planet_sphere(ray_dir, config.clouds_bottom_height);
    let end = min(intersect_planet_sphere(ray_dir, config.clouds_top_height), max_dist);

    if (start > max_dist) {
        return RaymarchResult(max_dist, vec4f(0.0, 0.0, 0.0, 1.0));
    }

    let ray_dot_sun = dot(ray_dir, -config.sun_dir.xyz);

    let step_distance = (end - start) / f32(config.clouds_raymarch_steps_count);
    let hashed_offset = common::hash13(ray_dir + fract(config.time));
    var dir_length = start - step_distance * hashed_offset;

    // Frostbite: dual-lobe phase function
    let scattering = mix(
        henyey_greenstein(ray_dot_sun, config.forward_scattering_g),
        henyey_greenstein(ray_dot_sun, config.backward_scattering_g),
        config.scattering_lerp
    );

    var scattered_light = vec3f(0.0, 0.0, 0.0);
    var transmittance = 1.0;

    var dist = config.planet_radius;

    for (var step: u32 = 0; step < config.clouds_raymarch_steps_count; step++) {
        let world_position = ray_origin + dir_length * ray_dir;

        let normalized_height = clamp(get_normalized_height(world_position), 0.0, 1.0);

        let clouds_density_sampled = get_cloud_map_density(world_position, normalized_height);

        if (clouds_density_sampled > 0.0) {
            dist = min(dist, dir_length);

            let ambient_light = mix(
                config.clouds_ambient_color_bottom,
                config.clouds_ambient_color_top,
                normalized_height
            );

            // Frostbite energy-conversing integration
            let S = clouds_density_sampled * (
                ambient_light.rgb +
                config.sun_color.rgb * scattering * volumetric_shadow(world_position, ray_dot_sun)
            );
            let delta_transmittance = exp(-clouds_density_sampled * step_distance);
            let integrated_scattering = S * (1.0 - delta_transmittance) / clouds_density_sampled;

            scattered_light += transmittance * integrated_scattering;
            transmittance *= delta_transmittance;
        }

        if transmittance <= config.clouds_min_transmittance { break; }

        dir_length += step_distance;
    }

    return RaymarchResult(dist, vec4f(scattered_light, transmittance));
}

// TODO: replace this by reading from Bevy's internal atmosphere rendering LUTs
// https://github.com/bevyengine/bevy/blob/v0.17.0/crates/bevy_pbr/src/atmosphere/functions.wgsl
fn get_sky_color(ray_dir: vec3f) -> vec3f {
    let mu = clamp(dot(ray_dir, config.sun_dir.xyz), 0.0, 1.0);
    let ray_dir_y = max(ray_dir.y, 0.01);
    let sky_color = vec3f(0.2, 0.5, 0.85);
    let horizon_strength = vec3f(0.0, 0.1, 0.1);

    // Sky
	var col = mix(
        sky_color - 0.5 * ray_dir_y * ray_dir_y,
        sky_color + vec3(0.5, 0.25, 0.0),
        pow(1.0 - ray_dir_y, 6.0)
    );

    // Horizon
    col += horizon_strength * clamp((1.0 - ray_dir.y * 10.0), 0.0, 1.0);

    // Sun
    col += 0.25 * config.sun_color.rgb * pow(mu, 6.0);
    col += 0.25 * config.sun_color.rgb * pow(mu, 64.0);
    col += 0.25 * config.sun_color.rgb * pow(mu, 512.0);

    return col;
}

fn render_clouds_atlas(frag_coord: vec2f) -> vec4f {
    let v_uv = frag_coord / config.render_resolution.xy;
    let coord = vec3f(v_uv, 0.5);

    let mfbm = 0.9;
    let mvor = 0.7;

    return vec4f(
        mix(1.0, common::tilable_perlin_fbm(coord, 7, 4), mfbm) *
            mix(1.0, common::tilable_voronoi(coord, 8, 9.0), mvor),
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

fn get_clouds_color(frag_coord: vec2f, camera: mat4x4f, old_cam: mat4x4f, ray_dir: vec3f, ray_origin: vec3f) -> vec4f {
    if (frag_coord.y < 1.5) {
        if frag_coord.x < 1.0 { return vec4f(config.render_resolution.xy, 0.0, 0.0); }
        return common::save_camera(camera, frag_coord, ray_origin);
    }

    let result = raymarch(ray_origin, ray_dir, 1e9);
    let transmittance = result.color.a;

    let fog_falloff = 1.0e-4;
    let fog_factor = 0.8 - exp(-fog_falloff * result.dist);

    let col = vec4f(
        mix(
            result.color.rgb,
            get_sky_color(ray_dir) * (1.0 - transmittance),
            fog_factor
        ),
        transmittance
    );

    // For now, just don't mix two frames when camera transform changed too much.
    // TODO: properly reproject old frame's reprojected pixel onto current frame.
    if length(abs(old_cam[0] - camera[0])) > EPSILON ||
        length(abs(old_cam[1] - camera[1])) > EPSILON ||
        length(abs(old_cam[2] - camera[2])) > EPSILON ||
        length(abs(old_cam[3] - camera[3])) > EPSILON {
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

    // Flip the Y co-ordinate from the clouds_top_height to the clouds_bottom_height to enter NDC.
    let ndc_xy = (rect_relative * 2.0 - vec2f(1.0, 1.0)) * vec2f(1.0, -1.0);

    let ray_clip = vec4f(ndc_xy.xy, -1.0, 1.0);
    let ray_eye = config.inverse_camera_projection * ray_clip;
    let ray_world = config.inverse_camera_view * vec4f(ray_eye.xy, -1.0, 0.0);

    return normalize(ray_world.xyz);
}

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let index = vec2f(f32(invocation_id.x), f32(invocation_id.y)) + vec2f(0.5);
    let inverted_y_coord = config.render_resolution.y - index.y;

    let worley_coord = vec2f(index.x, inverted_y_coord);

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
    let index = vec2f(f32(invocation_id.x), f32(invocation_id.y)) + vec2f(0.5);

    // Load old camera matrix before storageBarrier to prevent race conditions;
    let old_cam = common::load_camera(clouds_render_texture, u32(config.render_resolution.y) - 1);
    var frag_coord = vec2f(index.x, config.render_resolution.y - index.y);

    var ray_origin = get_ray_origin(config.time);
    var ray_dir = get_ray_direction(index);
    var col = get_clouds_color(frag_coord, config.inverse_camera_view, old_cam, ray_dir, ray_origin);
    let sky_color = vec4f(get_sky_color(ray_dir), 1.0);

    storageBarrier();

    textureStore(clouds_render_texture, invocation_id.xy, col);
    textureStore(sky_texture, invocation_id.xy, sky_color);
}
