#import bevy_sprite::{
    mesh2d_view_bindings::viewport,
    mesh2d_view_bindings::globals,
    mesh2d_functions::{get_world_from_local, mesh2d_position_local_to_clip},
}
#import bevy_pbr::{
    mesh_view_bindings::view,
    utils::coords_to_viewport_uv,
}

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(3) @binding(100) var clouds_render_texture: texture_2d<f32>;
@group(3) @binding(101) var clouds_render_sampler: sampler;

@group(1) @binding(102) var clouds_atlas_texture: texture_2d<f32>;
@group(1) @binding(103) var clouds_atlas_sampler: sampler;

@group(1) @binding(104) var clouds_worley_texture: texture_3d<f32>;
@group(1) @binding(105) var clouds_worley_sampler: sampler;

@group(3) @binding(106) var sky_texture: texture_2d<f32>;
@group(3) @binding(107) var sky_sampler: sampler;


@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let viewport_uv = coords_to_viewport_uv(mesh.position.xy, view.viewport);
    let clouds = textureSampleLevel(clouds_render_texture, clouds_render_sampler, vec2(viewport_uv), 0.0);
    let sky = textureSampleLevel(sky_texture, sky_sampler, vec2(viewport_uv), 0.0);

    return vec4(clouds.rgb + sky.rgb * clouds.a, 1.0);
}
