use bevy::prelude::*;
use food::FoodPlugin;
use movement::MovementPlugin;
use rand::{rngs::StdRng, SeedableRng};
use steering_agent::SteeringAgentPlugin;

mod movement;
mod steering_agent;
mod food;

#[derive(Resource)]
pub struct GameRng(StdRng);

impl GameRng {
    fn new(seed: u64) -> Self {
        Self(StdRng::seed_from_u64(seed))
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MovementPlugin,
            SteeringAgentPlugin,
            FoodPlugin
        ))
        .insert_resource(GameRng::new(42))
        .add_systems(Startup, setup)
        .add_systems(Update, exit_on_esc)
        .run();
}

fn setup(
    mut commands: Commands,
) {
    commands.spawn(Camera2d);
}

fn exit_on_esc(keys: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
    }
}