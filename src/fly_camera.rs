/// Camera movement logic based on <https://github.com/mcpar-land/bevy_fly_camera>
use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

/// A marker component used in queries when you want flycams and not other cameras
#[derive(Component)]
pub struct FlyCam;

/// Configuration for which keyboard keys control which action
/// Mouse is used for rotating the camera.
/// Keyboard is used for moving the camera around.
#[derive(Resource)]
pub struct KeyBindings {
    /// The [`KeyCode`] that increases the speed of the movement of the camera.
    pub speed_increase: KeyCode,
    /// The [`KeyCode`] that decreases the speed of the movement of the camera.
    pub speed_decrease: KeyCode,
    /// The [`KeyCode`] that moves the camera forward.
    pub move_forward: KeyCode,
    /// The [`KeyCode`] that moves the camera backward.
    pub move_backward: KeyCode,
    /// The [`KeyCode`] that moves the camera left.
    pub move_left: KeyCode,
    /// The [`KeyCode`] that moves the camera right.
    pub move_right: KeyCode,
    /// The [`KeyCode`] that moves the camera upward.
    pub move_ascend: KeyCode,
    /// The [`KeyCode`] that moves the camera downward.
    pub move_descend: KeyCode,
    /// The [`KeyCode`] that toggles between mouse (rotational) and keyboard (translational)
    /// control.
    pub toggle_grab_cursor: KeyCode,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            speed_increase: KeyCode::Digit2,
            speed_decrease: KeyCode::Digit1,
            move_forward: KeyCode::KeyW,
            move_backward: KeyCode::KeyS,
            move_left: KeyCode::KeyA,
            move_right: KeyCode::KeyD,
            move_ascend: KeyCode::Space,
            move_descend: KeyCode::ShiftLeft,
            toggle_grab_cursor: KeyCode::Space,
        }
    }
}

/// Mouse sensitivity controlling the rotation of the camera
#[derive(Resource)]
pub struct RotationSettings {
    /// Determines how sensitive the rotation is by moving the mouse
    pub sensitivity: f32,
}

impl Default for RotationSettings {
    fn default() -> Self {
        Self {
            sensitivity: 0.00010,
        }
    }
}

/// Mouse sensitivity and movement speed
#[derive(Resource)]
pub struct MovementSettings {
    /// Determines the speed of the camera movement in unit / second
    pub speed: f32,
}

impl Default for MovementSettings {
    fn default() -> Self {
        Self { speed: 1e3 }
    }
}

/// Handles keyboard input and movement
fn player_move(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    primary_cursor_options: Single<&CursorOptions, With<PrimaryWindow>>,
    settings: Option<Res<MovementSettings>>,
    key_bindings: Res<KeyBindings>,
    mut query: Query<&mut Transform, With<FlyCam>>,
) {
    if let Some(settings) = settings {
        for mut transform in query.iter_mut() {
            let mut velocity = Vec3::ZERO;
            let local_x = transform.local_x();
            let local_y = transform.local_y();
            let local_z = transform.local_z();
            let forward = -Vec3::new(local_z.x, local_z.y, local_z.z);
            let right = Vec3::new(local_x.x, local_x.y, local_x.z);
            let up = Vec3::new(local_y.x, local_y.y, local_y.z);

            for key in keys.get_pressed() {
                match primary_cursor_options.grab_mode {
                    CursorGrabMode::None => (),
                    _ => {
                        let key = *key;
                        if key == key_bindings.move_forward {
                            velocity += forward;
                        } else if key == key_bindings.move_backward {
                            velocity -= forward;
                        } else if key == key_bindings.move_left {
                            velocity -= right;
                        } else if key == key_bindings.move_right {
                            velocity += right;
                        } else if key == key_bindings.move_ascend {
                            velocity += up;
                        } else if key == key_bindings.move_descend {
                            velocity -= up;
                        }
                    }
                }
            }

            velocity = velocity.normalize_or_zero();

            transform.translation += velocity * time.delta_secs() * settings.speed;
        }
    }
}

/// Grabs/ungrabs mouse cursor
fn toggle_grab_cursor(mut primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    match primary_cursor_options.grab_mode {
        CursorGrabMode::None => {
            primary_cursor_options.grab_mode = CursorGrabMode::Confined;
            primary_cursor_options.visible = false;
        }
        _ => {
            primary_cursor_options.grab_mode = CursorGrabMode::None;
            primary_cursor_options.visible = true;
        }
    }
}

/// Handles looking around if cursor is locked
fn player_look(
    settings: Res<RotationSettings>,
    cursor_and_window: Single<(&CursorOptions, &Window), With<PrimaryWindow>>,
    mut state: MessageReader<MouseMotion>,
    mut query: Query<&mut Transform, With<FlyCam>>,
) {
    let (primary_cursor_options, window) = *cursor_and_window;
    for mut transform in query.iter_mut() {
        for ev in state.read() {
            let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
            match primary_cursor_options.grab_mode {
                CursorGrabMode::None => (),
                _ => {
                    // Using smallest of height or width ensures equal vertical and horizontal sensitivity
                    let window_scale = window.height().min(window.width());
                    pitch -= (settings.sensitivity * ev.delta.y * window_scale).to_radians();
                    yaw -= (settings.sensitivity * ev.delta.x * window_scale).to_radians();
                }
            }

            pitch = pitch.clamp(-1.54, 1.54);

            // Order is important to prevent unintended roll
            transform.rotation =
                Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch);
        }
    }
}

fn cursor_grab(
    keys: Res<ButtonInput<KeyCode>>,
    key_bindings: Res<KeyBindings>,
    primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    settings: Option<ResMut<MovementSettings>>,
) {
    if keys.just_pressed(key_bindings.toggle_grab_cursor) {
        toggle_grab_cursor(primary_cursor_options);
    }

    if let Some(mut settings) = settings {
        if keys.just_pressed(key_bindings.speed_increase) {
            settings.speed *= 2.0;
        }
        if keys.just_pressed(key_bindings.speed_decrease) {
            settings.speed /= 2.0;
        }
    }
}

/// A plugin to control a camera with "fly" keyboard controls
pub struct FlyCameraPlugin;
impl Plugin for FlyCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MovementSettings>()
            .init_resource::<RotationSettings>()
            .init_resource::<KeyBindings>()
            .add_systems(Update, (player_look, cursor_grab, player_move));
    }
}
