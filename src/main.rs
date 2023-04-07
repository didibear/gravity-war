use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    utils::{HashMap, HashSet},
    window::{close_on_esc, PresentMode},
};
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bevy_rapier2d::prelude::*;
use itertools::Itertools;

use bevy_prototype_debug_lines::*;

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
        .insert_resource(Spaceships::default())
        .init_resource::<Configuration>() // `ResourceInspectorPlugin` won't initialize the resource
        .register_type::<Configuration>() // you need to register your type to display it
        .add_plugin(ResourceInspectorPlugin::<Configuration>::default())
        .add_startup_systems((setup_graphics,))
        .add_systems((
            apply_forces,
            update_factions,
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

#[derive(Resource, Default)]
struct Spaceships {
    by_faction: HashMap<Faction, HashSet<Entity>>,
}

impl Spaceships {
    fn _in_other_factions(&self, current_faction: Faction) -> impl Iterator<Item = Entity> + '_ {
        self.by_faction
            .iter()
            .filter(move |(faction, _)| **faction != current_faction)
            .flat_map(|(_, entities)| entities.iter().copied())
    }
}

fn apply_forces(
    mut spaceship_forces: Query<(&Faction, &Transform, &mut ExternalForce), With<Spaceship>>,
    mut lines: ResMut<DebugLines>,
    _spaceships: Res<Spaceships>,
    configs: Res<Configuration>,
) {
    let spaceships_by_faction: HashMap<Faction, Vec<Vec3>> = spaceship_forces
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

    for (faction, transform, mut ext_force) in spaceship_forces.iter_mut() {
        if let Some((&closest_target, _target_distance)) = spaceships_by_faction
            .iter()
            .filter(|(target_faction, _)| *target_faction != faction)
            .flat_map(|(_, translations)| translations)
            .map(|target| (target, target.distance(transform.translation)))
            .min_by(|(_, a_distance), (_, b_distance)| a_distance.total_cmp(b_distance))
        {
            let target_direction = closest_target - transform.translation;
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
}

fn update_factions(
    mut spaceships: ResMut<Spaceships>,
    added: Query<(Entity, &Faction), Added<Faction>>,
    mut removed: RemovedComponents<Faction>,
) {
    for (entity, faction) in added.iter() {
        spaceships
            .by_faction
            .entry(*faction)
            .or_default()
            .insert(entity);
    }

    for entity in removed.iter() {
        for entities in spaceships.by_faction.values_mut() {
            entities.remove(&entity);
        }
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
        Spaceship::default(),
        Faction(faction),
        RigidBody::Dynamic,
        Collider::cuboid(10., 30.),
        Sensor,
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
