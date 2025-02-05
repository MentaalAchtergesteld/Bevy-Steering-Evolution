use std::f32::consts::TAU;

use bevy::{math::{vec2, VectorSpace}, prelude::*, window::PrimaryWindow};
use rand::Rng;

use crate::{movement::{Acceleration, MaxSpeed, Velocity, VelocityDamping}, GameRng};

pub struct SteeringAgentPlugin;

impl Plugin for SteeringAgentPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_initial_agents)
            .add_systems(Update, (follow_mouse, wander).chain());
    }
}

#[derive(Component)]
pub struct MaxForce(pub f32);
#[derive(Component)]
pub struct SlowingRadius(pub f32);

#[derive(Component)]
pub struct FollowMouse;

#[derive(Component)]
pub struct Wander {
    pub min_radius: f32,
    pub max_radius: f32,
    pub target: Vec2
}

impl Wander {
    pub fn new(origin: Vec2, min_radius: f32, max_radius: f32, rng: &mut impl Rng) -> Self {
        let mut wander = Wander {
            min_radius,
            max_radius,
            target: Vec2::ZERO,
        };

        wander.randomize(origin, rng);
    
        wander
    }

    pub fn randomize(&mut self, origin: Vec2, rng: &mut impl Rng) {
        let random_angle = rng.random_range(0.0..TAU);
        let random_distance = rng.random_range(self.min_radius..self.max_radius);

        self.target = vec2(
            origin.x + random_angle.cos() * random_distance,
            origin.y + random_angle.sin() * random_distance
        );
    }
}

#[derive(Bundle)]
pub struct SteeringAgentBundle {
    transform: Transform,
    velocity: Velocity,
    acceleration: Acceleration,
    damping: VelocityDamping,
    max_speed: MaxSpeed,
    max_force: MaxForce,
    slowing_radius: SlowingRadius
}

impl SteeringAgentBundle {
    pub fn new(position: Vec2, max_speed: f32, max_force: f32, slowing_radius: f32, damping: f32) -> Self {
        Self {
            transform: Transform::from_translation(position.extend(0.1)),
            velocity: Velocity::default(),
            acceleration: Acceleration::default(),
            damping: VelocityDamping(damping),
            max_speed: MaxSpeed(max_speed),
            max_force: MaxForce(max_force),
            slowing_radius: SlowingRadius(slowing_radius),
        }
    }
}

pub fn seek(
    current_pos: &Vec2,
    current_velocity: &Vec2, 
    target_pos: &Vec2,
    max_speed: f32,
    max_force: f32,
) -> Vec2 {
    let desired_velocity = (target_pos - current_pos).normalize_or_zero() * max_speed;

    let steering_force = (desired_velocity - current_velocity).clamp_length_max(max_force);

    steering_force
}

pub fn flee(
    current_pos: &Vec2,
    current_velocity: &Vec2,
    target_pos: &Vec2,
    max_speed: f32,
    max_force: f32,
) -> Vec2 {
    let desired_velocity = (current_pos - target_pos).normalize_or_zero() * max_speed;
    let steering_force = (desired_velocity - current_velocity).clamp_length_max(max_force);
    steering_force
}

pub fn arrive(
    current_pos: &Vec2,
    current_velocity: &Vec2,
    target_pos: &Vec2,
    max_speed: f32,
    max_force: f32,
    slowing_radius: f32,
) -> Vec2 {
    let distance = current_pos.distance(*target_pos);

    if distance < 0.1 {
        return Vec2::ZERO;
    }

    let desired_speed = if distance < slowing_radius {
        max_speed * (distance / slowing_radius)
    } else {
        max_speed
    };

    let desired_velocity = (target_pos - current_pos).normalize_or_zero() * desired_speed;
    
    let steering_force = (desired_velocity - current_velocity).clamp_length_max(max_force);

    steering_force
}

fn spawn_agent(
    position: Vec2,
    max_speed: f32,
    max_force: f32,
    slowing_radius: f32,
    damping: f32,
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
    commands: &mut Commands,
) -> Entity {
    commands.spawn((
        SteeringAgentBundle::new(position, max_speed, max_force, slowing_radius, damping),
        Mesh2d(mesh),
        MeshMaterial2d(material),
    )).id()
}

fn spawn_initial_agents(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut rng: ResMut<GameRng>
) {
    let triangle_width = 12.;
    let triangle_height = 18.;

    let triangle_top =   Vec2::new(2.0 * triangle_height / 3.0, 0.0);
    let triangle_left =  Vec2::new(-triangle_height / 3.0     , -triangle_width / 2.0);
    let triangle_right = Vec2::new(-triangle_height / 3.0     , triangle_width / 2.0);

    let agent_count = 8;

    for _ in 0..agent_count {
        let entity = spawn_agent(
            Vec2::ZERO,
            400.,
            1000.,
            50.,
            1.0,
            meshes.add(Triangle2d::new(triangle_top, triangle_left, triangle_right)),
            materials.add(Color::hsl(rng.0.random_range(0.0..360.0), 1., 0.75)),
            &mut commands
        );

        commands.get_entity(entity).unwrap().insert(Wander::new(Vec2::ZERO, 64., 512., &mut rng.0));
    }
}

fn follow_mouse(
    mut query: Query<(&mut Acceleration, &Velocity, &Transform, &MaxSpeed, &MaxForce, &SlowingRadius), With<FollowMouse>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    window_query: Query<&Window, With<PrimaryWindow>>
) {
    let (camera, camera_transform) = camera_query.single();
    let window = window_query.single();

    if let Some(viewport_position) = window.cursor_position() {
        let world_position = camera.viewport_to_world_2d(camera_transform, viewport_position).unwrap_or(Vec2::ZERO);

        for (mut acceleration, velocity, transform, max_speed, max_force, slowing_radius) in &mut query {
            acceleration.0 += arrive(
                &transform.translation.xy(),
                &velocity.0,
                &world_position,
                max_speed.0,
                max_force.0,
                slowing_radius.0
            );
        }
    }
}

fn wander(
    mut agent_query: Query<(&mut Acceleration, &mut Wander, &Velocity, &Transform, &MaxSpeed, &MaxForce, &SlowingRadius)>,
    mut rng: ResMut<GameRng>,
) {
    for (mut acceleration, mut wander, velocity, transform, max_speed, max_force, slowing_radius) in &mut agent_query {
        if transform.translation.xy().distance_squared(wander.target) < 1. {
            wander.randomize(Vec2::ZERO, &mut rng.0);
        }

        acceleration.0 += arrive(
            &transform.translation.xy(),
            &velocity.0,
            &wander.target,
            max_speed.0,
            max_force.0,
            slowing_radius.0
        );
    }
}