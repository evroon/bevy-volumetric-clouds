//! Bundles in our shader functions/consts etc declared in `common.wgsl` to make them importable.
use bevy::{
    asset::{Handle, load_internal_asset, uuid_handle},
    prelude::{App, Plugin, Shader},
};

const GLOBALS_TYPE_HANDLE: Handle<Shader> = uuid_handle!("0973cf27-0c9f-49a9-b818-4b927c013158");

pub(crate) struct ShaderCommonPlugin;

impl Plugin for ShaderCommonPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, GLOBALS_TYPE_HANDLE, "common.wgsl", Shader::from_wgsl);
    }
}
