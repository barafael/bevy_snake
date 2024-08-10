use bevy::{prelude::*, window::WindowResolution};

use rand::{prelude::random, Rng};

const SNAKE_HEAD_COLOR: Color = Color::srgb(0.7, 0.7, 0.7);
const FOOD_COLOR: Color = Color::srgb(1.0, 0.0, 1.0);
const SNAKE_SEGMENT_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);

const ARENA_HEIGHT: u32 = 10;
const ARENA_WIDTH: u32 = 10;

#[derive(Resource)]
struct FoodTimer(Timer);

#[derive(Resource)]
struct SnakeMovementTimer(Timer);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

#[derive(Component)]
struct SnakeHead {
    direction: Direction,
}

#[derive(Event)]
struct GameOverEvent;

#[derive(Event)]
struct GrowthEvent;

#[derive(Resource, Default)]
struct LastTailPosition(Option<Position>);

#[derive(Component)]
struct SnakeSegment;

#[derive(Resource, Default)]
struct SnakeSegments(Vec<Entity>);

#[derive(Component)]
struct Food;

#[derive(PartialEq, Copy, Clone)]
enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

#[derive(Component)]
struct MyCameraMarker;

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), MyCameraMarker));
}

fn spawn_snake(mut commands: Commands, mut segments: ResMut<SnakeSegments>) {
    *segments = SnakeSegments(vec![
        commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    color: SNAKE_HEAD_COLOR,
                    ..default()
                },
                ..default()
            })
            .insert(SnakeHead {
                direction: Direction::Up,
            })
            .insert(SnakeSegment)
            .insert(Position { x: 3, y: 3 })
            .insert(Size::square(0.8))
            .id(),
        spawn_segment(commands, Position { x: 3, y: 2 }),
    ]);
}

fn spawn_segment(mut commands: Commands, position: Position) -> Entity {
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: SNAKE_SEGMENT_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(SnakeSegment)
        .insert(position)
        .insert(Size::square(0.65))
        .id()
}

fn snake_movement(
    time: Res<Time>,
    mut timer: ResMut<SnakeMovementTimer>,
    mut last_tail_position: ResMut<LastTailPosition>,
    mut game_over_writer: EventWriter<GameOverEvent>,
    segments: ResMut<SnakeSegments>,
    mut heads: Query<(Entity, &SnakeHead)>,
    mut positions: Query<&mut Position>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        if let Ok((head_entity, head)) = heads.get_single_mut() {
            let segment_positions = segments
                .0
                .iter()
                .map(|e| *positions.get_mut(*e).unwrap())
                .collect::<Vec<Position>>();
            let mut head_pos = positions.get_mut(head_entity).unwrap();
            match &head.direction {
                Direction::Left => head_pos.x -= 1,
                Direction::Up => head_pos.y += 1,
                Direction::Right => head_pos.x += 1,
                Direction::Down => head_pos.y -= 1,
            }
            if head_pos.x < 0
                || head_pos.y < 0
                || head_pos.x as u32 >= ARENA_WIDTH
                || head_pos.y as u32 >= ARENA_HEIGHT
            {
                game_over_writer.send(GameOverEvent);
            }
            if segment_positions.contains(&head_pos) {
                game_over_writer.send(GameOverEvent);
            }
            segment_positions
                .iter()
                .zip(segments.0.iter().skip(1))
                .for_each(|(pos, segment)| {
                    *positions.get_mut(*segment).unwrap() = *pos;
                });
            *last_tail_position = LastTailPosition(Some(*segment_positions.last().unwrap()));
        }
    }
}

fn snake_movement_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut heads: Query<&mut SnakeHead>,
) {
    if let Ok(mut head) = heads.get_single_mut() {
        let dir: Direction = if keyboard_input.pressed(KeyCode::ArrowLeft) {
            Direction::Left
        } else if keyboard_input.pressed(KeyCode::ArrowDown) {
            Direction::Down
        } else if keyboard_input.pressed(KeyCode::ArrowUp) {
            Direction::Up
        } else if keyboard_input.pressed(KeyCode::ArrowRight) {
            Direction::Right
        } else {
            head.direction
        };
        if dir != head.direction.opposite() {
            head.direction = dir;
        }
    }
}

fn game_over(
    mut commands: Commands,
    mut reader: EventReader<GameOverEvent>,
    segments_res: ResMut<SnakeSegments>,
    food: Query<Entity, With<Food>>,
    segments: Query<Entity, With<SnakeSegment>>,
) {
    if reader.read().next().is_some() {
        for ent in food.iter().chain(segments.iter()) {
            commands.entity(ent).despawn();
        }
        spawn_snake(commands, segments_res);
    }
}

fn snake_eating(
    mut commands: Commands,
    mut growth_writer: EventWriter<GrowthEvent>,
    food_positions: Query<(Entity, &Position), With<Food>>,
    head_positions: Query<&Position, With<SnakeHead>>,
) {
    for head_pos in head_positions.iter() {
        for (ent, food_pos) in food_positions.iter() {
            if food_pos == head_pos {
                commands.entity(ent).despawn();
                growth_writer.send(GrowthEvent);
            }
        }
    }
}

fn snake_growth(
    commands: Commands,
    last_tail_position: Res<LastTailPosition>,
    mut segments: ResMut<SnakeSegments>,
    mut growth_reader: EventReader<GrowthEvent>,
) {
    if growth_reader.read().next().is_some() {
        segments
            .0
            .push(spawn_segment(commands, last_tail_position.0.unwrap()));
    }
}

fn size_scaling(windows: Query<&Window>, mut q: Query<(&Size, &mut Transform)>) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    for (sprite_size, mut transform) in &mut q {
        transform.scale = Vec3::new(
            sprite_size.width / ARENA_WIDTH as f32 * window.width(),
            sprite_size.height / ARENA_HEIGHT as f32 * window.height(),
            1.0,
        );
    }
}

fn position_translation(windows: Query<&Window>, mut q: Query<(&Position, &mut Transform)>) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    for (pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width(), ARENA_WIDTH as f32),
            convert(pos.y as f32, window.height(), ARENA_HEIGHT as f32),
            0.0,
        );
    }
}

fn food_spawner(time: Res<Time>, mut timer: ResMut<FoodTimer>, mut commands: Commands) {
    if timer.0.tick(time.delta()).just_finished() {
        let mut rng = rand::thread_rng();
        commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    color: FOOD_COLOR,
                    ..default()
                },
                ..Default::default()
            })
            .insert(Food)
            .insert(Position {
                x: rng.gen_range(0..ARENA_WIDTH as i32),
                y: rng.gen_range(0..ARENA_HEIGHT as i32),
            })
            .insert(Size::square(0.8));
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.04, 0.04, 0.04)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(500., 500.).with_scale_factor_override(1.0),
                title: "Snake!".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, spawn_snake)
        .insert_resource(SnakeSegments::default())
        .insert_resource(LastTailPosition::default())
        .add_event::<GrowthEvent>()
        .add_systems(
            Update,
            (
                snake_movement_input,
                snake_movement,
                game_over,
                snake_eating,
                snake_growth,
            )
                .chain(),
        )
        .add_event::<GameOverEvent>()
        .add_systems(Update, food_spawner)
        .insert_resource(FoodTimer(Timer::from_seconds(2.0, TimerMode::Repeating)))
        .insert_resource(SnakeMovementTimer(Timer::from_seconds(
            0.150,
            TimerMode::Repeating,
        )))
        .add_systems(PostUpdate, (position_translation, size_scaling))
        .run();
}
