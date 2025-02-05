use std::f32::consts::TAU;

use bevy::{color::palettes::css::GREEN, prelude::*, window::PrimaryWindow};
use rand::Rng;
use rand_distr::{Distribution, Exp};

use crate::{movement::Velocity, GameRng};

pub struct FoodPlugin;

impl Plugin for FoodPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, (setup_food_assets, spawn_initial_food).chain())
            .add_systems(Update, (food_duplication, food_velocity_damping, food_cohesion));
    }
}

#[derive(Resource)]
struct FoodAssets {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>
}

#[derive(Component, Default, Clone, Copy)]
pub struct Food {
    pub nutritional_value: f32,

    pub duplication_chance: f32,
    pub spawn_velocity_min: f32,
    pub spawn_velocity_max: f32,
    pub cohesion_radius: f32,
    pub cohesion_force: f32,
    pub seperation_radius: f32,
    pub seperation_force: f32,

    pub neighbour_radius: f32,
    pub max_neighbours: usize,
}

#[derive(Bundle)]
struct FoodBundle {
    food: Food,
    mesh: Mesh2d,
    material: MeshMaterial2d<ColorMaterial>,
    transform: Transform,
    velocity: Velocity
}

impl FoodBundle {
    fn from_food(food: Food, position: Vec2, initial_velocity: Vec2, mesh: Handle<Mesh>, material: Handle<ColorMaterial>) -> Self {
        FoodBundle {
            food: food,
            transform: Transform::from_translation(position.extend(0.)),
            mesh: Mesh2d(mesh),
            material: MeshMaterial2d(material),
            velocity: Velocity(initial_velocity)
        }
    }
}

fn setup_food_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(FoodAssets {
        mesh: meshes.add(Circle::new(6.0)),
        material: materials.add(ColorMaterial::from_color(GREEN))
    });
}

fn spawn_food(
    food: Food,
    position: Vec2,
    commands: &mut Commands,
    rng: &mut impl Rng,
    food_assets: &Res<FoodAssets>
) {
    let initial_force = rng.random_range(food.spawn_velocity_min..food.spawn_velocity_max);
    let initial_angle = rng.random_range(0.0..TAU);

    let initial_velocity = Vec2::new(
        initial_angle.cos() * initial_force,
        initial_angle.sin() * initial_force,
    );

    commands.spawn(FoodBundle::from_food(
        food,
        position,
        initial_velocity,
        food_assets.mesh.clone(),
        food_assets.material.clone()
    ));
}

fn spawn_initial_food(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    food_assets: Res<FoodAssets>,
    window_query: Query<&Window, With<PrimaryWindow>>
) {
    let window = window_query.single();

    let half_window_size = window.size() / 2.;

    let base_food = Food {
        nutritional_value: 4.0,
        duplication_chance: 0.1,
        spawn_velocity_min: 5.0,
        spawn_velocity_max: 100.0,
        cohesion_radius: 64.0,
        cohesion_force: 4.0,
        seperation_radius: 32.0,
        seperation_force: 256.0,
        neighbour_radius: 64.0,
        max_neighbours: 16,
    };

    let food_count = 4;

    for _ in 0..food_count {
        let position = Vec2::new(
            rng.0.random_range(-half_window_size.x..half_window_size.x),
            rng.0.random_range(-half_window_size.y..half_window_size.y),
        );

        let mut new_food = base_food.clone();
        new_food.nutritional_value *= rng.0.random_range(0.8..1.2);

        spawn_food(new_food, position, &mut commands, &mut rng.0, &food_assets);
    }
}

fn count_nearby_food(
    positions_query: &Query<&Transform, With<Food>>,
    position: Vec2,
    radius: f32
) -> usize {
    positions_query
        .iter()
        .filter(|&transform| {
            position.distance_squared(transform.translation.xy()) < radius * radius
        })
        .count()
}

fn food_duplication(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    food_query: Query<(&Transform, &Food)>,
    positions_query: Query<&Transform, With<Food>>,
    food_assets: Res<FoodAssets>,
    time: Res<Time>,
) {
    for (transform, food) in &food_query {
        let nearby_food = count_nearby_food(&positions_query, transform.translation.xy(), food.neighbour_radius);

        if nearby_food < food.max_neighbours {
            let chance = food.duplication_chance * time.delta_secs();

            if rng.0.random_bool(chance as f64) {
                let position = transform.translation.xy().clone();
    
                let new_food = food.clone();
                spawn_food(new_food, position, &mut commands, &mut rng.0, &food_assets);
            }
        }
    }
}

fn food_velocity_damping(
    mut query: Query<&mut Velocity, With<Food>>,
    time: Res<Time>
) {
    for mut velocity in &mut query {
        let damping_factor = (1.0 - 2.0 * time.delta_secs()).max(0.0);
        velocity.0 *= damping_factor;

        if velocity.0.length_squared() < 1e-5 {
            velocity.0 = Vec2::ZERO;
        }
    }
}

fn food_cohesion(
    mut query: Query<(&Transform, &mut Velocity, &Food)>,
    time: Res<Time>
) {
    let mut food_iter = query.iter_combinations_mut();

    while let Some([(transform_a, mut velocity_a, food_a), (transform_b, ..)]) = food_iter.fetch_next() {
        let delta = (transform_b.translation - transform_a.translation).xy();
        let distance = delta.length();

        if distance < f32::EPSILON {
            continue;
        }

        if distance < food_a.cohesion_radius {
            let attraction_force = delta.normalize_or_zero() * food_a.cohesion_force * time.delta_secs();
            velocity_a.0 += attraction_force;
        }

        if distance < food_a.seperation_radius {
            let repulsion_force = delta.normalize_or_zero() * -food_a.seperation_force * time.delta_secs();
            velocity_a.0 += repulsion_force / distance.max(1.0);
        }
    }
}