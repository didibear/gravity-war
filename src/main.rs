use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    sprite::MaterialMesh2dBundle,
    utils::{HashMap, HashSet},
    window::{close_on_esc, PresentMode},
};
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bevy_rapier2d::prelude::*;
use itertools::Itertools;

use bevy_prototype_debug_lines::*;
use rand::rngs::SmallRng;
use rand::Rng;
use rand::SeedableRng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gravity War".into(),
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(DebugLinesPlugin::default())
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<Configuration>() // `ResourceInspectorPlugin` won't initialize the resource
        .register_type::<Configuration>() // you need to register your type to display it
        .add_plugin(ResourceInspectorPlugin::<Configuration>::default())
        .add_startup_systems((setup_graphics, spawn_stars))
        .add_systems((
            update_targets,
            apply_forces.after(update_targets),
            camera_follow_spaceships,
            close_on_esc,
            move_spaceship,
            spawn_by_click,
        ))
        .run();
}

#[derive(Reflect, Resource, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct Configuration {
    rotation_force: f32,
    propulsion_force: f32,
    aim_distance: f32,
    rotation_max: f32,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            rotation_force: 0.02,
            propulsion_force: 50.,
            aim_distance: 100.,
            rotation_max: 0.05,
        }
    }
}

#[derive(Component, Default)]
struct Spaceship;

#[derive(Component, Default)]
struct Target {
    translation: Vec3,
    distance: f32,
}

#[derive(Component, Hash, Clone, Copy, PartialEq, Eq)]
struct Faction(pub u32);

impl From<Faction> for Color {
    fn from(value: Faction) -> Self {
        const COLORS: [Color; 5] = [
            Color::BLUE,
            Color::RED,
            Color::GREEN,
            Color::YELLOW,
            Color::PURPLE,
        ];
        COLORS[value.0 as usize]
    }
}

fn setup_graphics(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn update_targets(mut targets: Query<(&Faction, &Transform, &mut Target)>) {
    let targets_by_faction: HashMap<Faction, Vec<Vec3>> = targets
        .iter()
        .group_by(|(faction, ..)| **faction)
        .into_iter()
        .map(|(faction, group)| {
            (
                faction,
                group
                    .map(|(_, transform, _)| transform.translation)
                    .collect(),
            )
        })
        .collect();

    for (faction, transform, mut target) in targets.iter_mut() {
        if let Some((&closest_target, target_distance)) = targets_by_faction
            .iter()
            .filter(|(target_faction, _)| *target_faction != faction)
            .flat_map(|(_, translations)| translations)
            .map(|target| (target, target.distance(transform.translation)))
            .min_by(|(_, a_distance), (_, b_distance)| a_distance.total_cmp(b_distance))
        {
            *target = Target {
                translation: closest_target,
                distance: target_distance,
            };
        }
    }
}

fn apply_forces(
    mut spaceship_forces: Query<
        (&Faction, &Target, &Transform, &mut ExternalForce),
        With<Spaceship>,
    >,
    mut lines: ResMut<DebugLines>,
    configs: Res<Configuration>,
) {
    for (faction, target, transform, mut ext_force) in spaceship_forces.iter_mut() {
        let target_direction = target.translation - transform.translation;
        let direction = transform.up();

        let angle = direction
            .truncate()
            .angle_between(target_direction.truncate());

        ext_force.torque =
            (angle * configs.rotation_force).clamp(-configs.rotation_max, configs.rotation_max);
        ext_force.force = (direction * configs.propulsion_force)
                .truncate()
                // .clamp_length_min(target_distance )
                ;

        let pos = transform.translation;
        lines.line_colored(pos, pos + direction * 100., 0., Color::from(*faction));
        // lines.line_colored(pos, pos + target_direction * 0.1, 0., Color::YELLOW);
    }
}

fn move_spaceship(
    keyboard: Res<Input<KeyCode>>,
    mut spaceships: Query<(&mut Transform, &Faction), With<Spaceship>>,
    time: Res<Time>,
) {
    for (mut transform, faction) in spaceships.iter_mut() {
        if *faction != Faction(1) {
            continue;
        }
        let speed: f32 = 1000. * time.delta_seconds();
        if keyboard.pressed(KeyCode::Up) {
            transform.translation += Vec3::Y * speed;
        }
        if keyboard.pressed(KeyCode::Down) {
            transform.translation -= Vec3::Y * speed;
        }
        if keyboard.pressed(KeyCode::Left) {
            transform.translation -= Vec3::X * speed;
        }
        if keyboard.pressed(KeyCode::Right) {
            transform.translation += Vec3::X * speed;
        }
    }
}

fn spawn_by_click(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    mut cursor_moved_events: EventReader<CursorMoved>,
) {
    let faction_to_spawn = {
        if mouse_button_input.just_pressed(MouseButton::Left) {
            Some(1)
        } else if mouse_button_input.just_pressed(MouseButton::Right) {
            Some(2)
        } else {
            None
        }
    };

    if let Some(faction) = faction_to_spawn {
        if let Some(event) = cursor_moved_events.iter().last() {
            commands.spawn(spaceship_bundle(
                faction,
                event.position.x - 1280. / 2.,
                event.position.y - 720. / 2.,
            ));
        }
    }
}

fn spaceship_bundle(faction: u32, x: f32, y: f32) -> impl Bundle {
    (
        Spaceship,
        Faction(faction),
        Target::default(),
        // Physic
        RigidBody::Dynamic,
        Sensor,
        Collider::cuboid(10., 30.),
        Restitution::coefficient(0.7),
        ExternalForce::default(),
        GravityScale(0.),
        Damping {
            linear_damping: 1.,
            angular_damping: 2.,
        },
        TransformBundle::from(Transform::from_xyz(x, y, 0.0)),
    )
}

fn camera_follow_spaceships(
    mut camera: Query<&mut Transform, With<Camera>>,
    spaceships: Query<&Transform, (With<Spaceship>, Without<Camera>)>,
) {
    let count = spaceships.iter().len();
    if count == 0 {
        return;
    }

    let translations = spaceships.iter().map(|t| t.translation.truncate());
    let avg_translation = translations.sum::<Vec2>() / count as f32;

    let mut camera_transform = camera.single_mut();
    camera_transform.translation.x = avg_translation.x;
    camera_transform.translation.y = avg_translation.y;
}

fn spawn_stars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut rng = SmallRng::seed_from_u64(42);

    let mesh = meshes.add(shape::Circle::new(1.).into());
    let material = materials.add(ColorMaterial::from(Color::WHITE));

    for _ in 0..100 {
        let x = rng.gen_range(-1000.0..1000.0);
        let y = rng.gen_range(-1000.0..1000.0);

        commands.spawn(MaterialMesh2dBundle {
            mesh: mesh.clone().into(),
            material: material.clone(),
            transform: Transform::from_translation(Vec3::new(x, y, 0.)),
            ..default()
        });
    }
}
