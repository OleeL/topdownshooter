use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh2d};
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::sprite_render::MeshMaterial2d;
use bevy::window::WindowResolution;
use std::f32::consts::PI;

const WIDTH: f32 = 960.0;
const HEIGHT: f32 = 640.0;
const PLAYER_RADIUS: f32 = 15.0;
const PLAYER_SPEED: f32 = 230.0;
const PLAYER_SPRINT_SPEED: f32 = 330.0;
const LIGHT_RADIUS: f32 = 620.0;
const BULLET_SPEED: f32 = 720.0;
const BULLET_RADIUS: f32 = 4.0;
const ENEMY_RADIUS: f32 = 14.0;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.02, 0.025, 0.035)))
        .insert_resource(Score(0))
        .insert_resource(SpawnTimer(Timer::from_seconds(1.1, TimerMode::Repeating)))
        .insert_resource(GameOver(false))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Ray Trace Shooter".into(),
                resolution: WindowResolution::new(WIDTH as u32, HEIGHT as u32),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                restart,
                player_input,
                shoot,
                update_bullets,
                spawn_enemies,
                update_enemies,
                update_enemy_visibility,
                update_lighting,
                update_hud,
            ),
        )
        .run();
}

#[derive(Component)]
struct Player {
    health: f32,
    aim: Vec2,
    fire_timer: Timer,
}

#[derive(Component)]
struct Enemy {
    health: i32,
    speed: f32,
    bite_timer: Timer,
}

#[derive(Component)]
struct Bullet {
    velocity: Vec2,
    lifetime: Timer,
}

#[derive(Component, Clone, Copy)]
struct Wall {
    half: Vec2,
}

#[derive(Component)]
struct LightMask;

#[derive(Component)]
struct Hud;

#[derive(Component)]
struct Body;

#[derive(Component)]
struct Splat;

#[derive(Resource)]
struct Score(u32);

#[derive(Resource)]
struct SpawnTimer(Timer);

#[derive(Resource)]
struct GameOver(bool);

#[derive(Clone, Copy)]
struct Segment {
    a: Vec2,
    b: Vec2,
}

#[derive(Clone, Copy)]
struct Rect {
    centre: Vec2,
    half: Vec2,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite::from_color(Color::srgb(0.07, 0.08, 0.1), Vec2::new(WIDTH, HEIGHT)),
        Transform::from_xyz(0.0, 0.0, -20.0),
    ));

    for x in (-480..=480).step_by(40) {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.1, 0.11, 0.13), Vec2::new(1.0, HEIGHT)),
            Transform::from_xyz(x as f32, 0.0, -19.0),
        ));
    }

    for y in (-320..=320).step_by(40) {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.1, 0.11, 0.13), Vec2::new(WIDTH, 1.0)),
            Transform::from_xyz(0.0, y as f32, -19.0),
        ));
    }

    for rect in level_walls() {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.34, 0.34, 0.37), rect.half * 2.0),
            Transform::from_xyz(rect.centre.x, rect.centre.y, -5.0),
            Wall { half: rect.half },
        ));
    }

    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.25, 0.85, 1.0),
            Vec2::splat(PLAYER_RADIUS * 2.0),
        ),
        Transform::from_xyz(0.0, -180.0, 2.0),
        Player {
            health: 100.0,
            aim: Vec2::X,
            fire_timer: Timer::from_seconds(0.12, TimerMode::Once),
        },
    ));

    let mesh = meshes.add(Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    ));
    commands.spawn((
        Mesh2d(mesh),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgba(0.0, 0.0, 0.0, 0.62)))),
        Transform::from_xyz(0.0, 0.0, 20.0),
        LightMask,
    ));

    commands.spawn((
        Text::new("WASD move  Mouse aim  Hold LMB shoot  Shift sprint"),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(10.0),
            ..default()
        },
        Hud,
    ));
}

fn restart(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut game_over: ResMut<GameOver>,
    mut score: ResMut<Score>,
    mut players: Query<(&mut Transform, &mut Player)>,
    enemies: Query<Entity, With<Enemy>>,
    bullets: Query<Entity, With<Bullet>>,
    bodies: Query<Entity, Or<(With<Body>, With<Splat>)>>,
) {
    if !game_over.0 || !keys.just_pressed(KeyCode::KeyR) {
        return;
    }

    game_over.0 = false;
    score.0 = 0;

    for entity in enemies.iter().chain(bullets.iter()).chain(bodies.iter()) {
        commands.entity(entity).despawn();
    }

    if let Ok((mut transform, mut player)) = players.single_mut() {
        transform.translation = Vec3::new(0.0, -180.0, 2.0);
        player.health = 100.0;
        player.aim = Vec2::X;
    }
}

fn player_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    walls: Query<(&Transform, &Wall)>,
    game_over: Res<GameOver>,
    mut players: Query<(&mut Transform, &mut Player), Without<Wall>>,
) {
    if game_over.0 {
        return;
    }

    let Ok((mut transform, mut player)) = players.single_mut() else {
        return;
    };
    let mut movement = Vec2::ZERO;

    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        movement.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        movement.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        movement.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        movement.x += 1.0;
    }

    if movement.length_squared() > 0.0 {
        movement = movement.normalize();
    }

    let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        PLAYER_SPRINT_SPEED
    } else {
        PLAYER_SPEED
    };

    move_circle(
        &mut transform.translation,
        movement * speed * time.delta_secs(),
        PLAYER_RADIUS,
        &walls,
    );

    if let Some(cursor) = cursor_world(&windows, &camera) {
        let aim = cursor - transform.translation.truncate();
        if aim.length_squared() > 0.0 {
            player.aim = aim.normalize();
        }
    }

    player.fire_timer.tick(time.delta());
}

fn shoot(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    game_over: Res<GameOver>,
    mut players: Query<(&Transform, &mut Player)>,
) {
    if game_over.0 || !buttons.pressed(MouseButton::Left) {
        return;
    }

    let Ok((transform, mut player)) = players.single_mut() else {
        return;
    };
    if !player.fire_timer.is_finished() {
        return;
    }

    player.fire_timer.reset();
    let pos = transform.translation.truncate() + player.aim * (PLAYER_RADIUS + 6.0);
    commands.spawn((
        Sprite::from_color(
            Color::srgb(1.0, 0.86, 0.35),
            Vec2::splat(BULLET_RADIUS * 2.0),
        ),
        Transform::from_xyz(pos.x, pos.y, 4.0),
        Bullet {
            velocity: player.aim * BULLET_SPEED,
            lifetime: Timer::from_seconds(1.2, TimerMode::Once),
        },
    ));
}

fn update_bullets(
    time: Res<Time>,
    mut commands: Commands,
    mut score: ResMut<Score>,
    walls: Query<(&Transform, &Wall)>,
    mut bullets: Query<(Entity, &mut Transform, &mut Bullet), (Without<Enemy>, Without<Wall>)>,
    mut enemies: Query<(Entity, &mut Transform, &mut Enemy), (Without<Bullet>, Without<Wall>)>,
) {
    for (bullet_entity, mut bullet_transform, mut bullet) in &mut bullets {
        bullet.lifetime.tick(time.delta());
        bullet_transform.translation += bullet.velocity.extend(0.0) * time.delta_secs();
        let bullet_pos = bullet_transform.translation.truncate();
        let mut remove = bullet.lifetime.is_finished()
            || bullet_pos.x.abs() > WIDTH * 0.5
            || bullet_pos.y.abs() > HEIGHT * 0.5
            || circle_hits_wall(bullet_pos, BULLET_RADIUS, &walls);

        if !remove {
            for (enemy_entity, mut enemy_transform, mut enemy) in &mut enemies {
                let enemy_pos = enemy_transform.translation.truncate();
                if bullet_pos.distance(enemy_pos) < BULLET_RADIUS + ENEMY_RADIUS {
                    let pushed = enemy_pos + bullet.velocity.normalize_or_zero() * 12.0;
                    if !circle_hits_wall(pushed, ENEMY_RADIUS, &walls) {
                        enemy_transform.translation.x = pushed.x;
                        enemy_transform.translation.y = pushed.y;
                    }

                    enemy.health -= 1;
                    remove = true;

                    if enemy.health <= 0 {
                        spawn_body_and_splat(&mut commands, enemy_transform.translation.truncate());
                        commands.entity(enemy_entity).despawn();
                        score.0 += 10;
                    }
                    break;
                }
            }
        }

        if remove {
            commands.entity(bullet_entity).despawn();
        }
    }
}

fn spawn_enemies(
    time: Res<Time>,
    mut commands: Commands,
    mut timer: ResMut<SpawnTimer>,
    score: Res<Score>,
    game_over: Res<GameOver>,
) {
    if game_over.0 {
        return;
    }

    timer.0.tick(time.delta());
    if !timer.0.is_finished() {
        return;
    }

    timer.0.set_duration(std::time::Duration::from_secs_f32(
        (1.3 - score.0 as f32 * 0.006).max(0.45),
    ));

    let pos = match rand_index(4) {
        0 => Vec2::new(random_range(-430.0, 430.0), 280.0),
        1 => Vec2::new(random_range(-430.0, 430.0), -280.0),
        2 => Vec2::new(-430.0, random_range(-280.0, 280.0)),
        _ => Vec2::new(430.0, random_range(-280.0, 280.0)),
    };

    commands.spawn((
        Sprite::from_color(
            Color::srgb(1.0, 0.18, 0.16),
            Vec2::splat(ENEMY_RADIUS * 2.0),
        ),
        Transform::from_xyz(pos.x, pos.y, 3.0),
        Visibility::Hidden,
        Enemy {
            health: 2,
            speed: random_range(75.0, 115.0),
            bite_timer: Timer::from_seconds(0.55, TimerMode::Once),
        },
    ));
}

fn update_enemies(
    time: Res<Time>,
    mut game_over: ResMut<GameOver>,
    walls: Query<(&Transform, &Wall)>,
    mut players: Query<(&Transform, &mut Player), (Without<Enemy>, Without<Wall>)>,
    mut enemies: Query<(&mut Transform, &mut Enemy), Without<Wall>>,
) {
    if game_over.0 {
        return;
    }

    let Ok((player_transform, mut player)) = players.single_mut() else {
        return;
    };
    let player_pos = player_transform.translation.truncate();

    for (mut enemy_transform, mut enemy) in &mut enemies {
        let enemy_pos = enemy_transform.translation.truncate();
        let direction = (player_pos - enemy_pos).normalize_or_zero();
        move_circle(
            &mut enemy_transform.translation,
            direction * enemy.speed * time.delta_secs(),
            ENEMY_RADIUS,
            &walls,
        );

        enemy.bite_timer.tick(time.delta());
        if enemy_transform.translation.truncate().distance(player_pos)
            < PLAYER_RADIUS + ENEMY_RADIUS + 4.0
            && enemy.bite_timer.is_finished()
        {
            player.health -= 12.0;
            enemy.bite_timer.reset();
            if player.health <= 0.0 {
                game_over.0 = true;
            }
        }
    }
}

fn update_enemy_visibility(
    players: Query<&Transform, With<Player>>,
    walls: Query<(&Transform, &Wall)>,
    mut enemies: Query<(&Transform, &mut Visibility), With<Enemy>>,
) {
    let Ok(player_transform) = players.single() else {
        return;
    };
    let player_pos = player_transform.translation.truncate();
    let segments = wall_segments(&walls);

    for (enemy_transform, mut visibility) in &mut enemies {
        let enemy_pos = enemy_transform.translation.truncate();
        *visibility = if in_sight(player_pos, enemy_pos, &segments) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_lighting(
    players: Query<&Transform, With<Player>>,
    walls: Query<(&Transform, &Wall)>,
    mut meshes: ResMut<Assets<Mesh>>,
    masks: Query<&Mesh2d, With<LightMask>>,
) {
    let Ok(player_transform) = players.single() else {
        return;
    };
    let Ok(mesh_2d) = masks.single() else {
        return;
    };
    let polygon = visibility_polygon(
        player_transform.translation.truncate(),
        LIGHT_RADIUS,
        &wall_segments(&walls),
    );

    if let Some(mut mesh) = meshes.get_mut(&mesh_2d.0) {
        *mesh = darkness_mesh(&polygon);
    }
}

fn update_hud(
    score: Res<Score>,
    game_over: Res<GameOver>,
    players: Query<&Player>,
    mut hud: Query<&mut Text, With<Hud>>,
) {
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let health = players
        .single()
        .map_or(0.0, |player| player.health.max(0.0));
    let end = if game_over.0 {
        "\nGAME OVER - press R to restart"
    } else {
        ""
    };
    text.0 = format!(
        "WASD move  Mouse aim  Hold LMB shoot  Shift sprint\nScore: {}  Health: {:.0}{}",
        score.0, health, end
    );
}

fn spawn_body_and_splat(commands: &mut Commands, pos: Vec2) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.42, 0.05, 0.055), Vec2::new(32.0, 22.0)),
        Transform::from_xyz(pos.x, pos.y, 1.0)
            .with_rotation(Quat::from_rotation_z(random_range(0.0, PI * 2.0))),
        Body,
    ));

    for _ in 0..rand_index(6) + 7 {
        let offset = Vec2::from_angle(random_range(0.0, PI * 2.0)) * random_range(6.0, 26.0);
        let size = random_range(6.0, 18.0);
        commands.spawn((
            Sprite::from_color(Color::srgba(0.35, 0.0, 0.015, 0.78), Vec2::splat(size)),
            Transform::from_xyz(pos.x + offset.x, pos.y + offset.y, 0.5),
            Splat,
        ));
    }
}

fn move_circle(pos: &mut Vec3, delta: Vec2, radius: f32, walls: &Query<(&Transform, &Wall)>) {
    let mut next = pos.truncate();
    next.x += delta.x;
    if !circle_hits_wall(next, radius, walls) {
        pos.x = next.x;
    }

    next = pos.truncate();
    next.y += delta.y;
    if !circle_hits_wall(next, radius, walls) {
        pos.y = next.y;
    }
}

fn circle_hits_wall(pos: Vec2, radius: f32, walls: &Query<(&Transform, &Wall)>) -> bool {
    walls.iter().any(|(transform, wall)| {
        circle_rect_overlap(
            pos,
            radius,
            Rect {
                centre: transform.translation.truncate(),
                half: wall.half,
            },
        )
    })
}

fn circle_rect_overlap(pos: Vec2, radius: f32, rect: Rect) -> bool {
    let min = rect.centre - rect.half;
    let max = rect.centre + rect.half;
    let closest = Vec2::new(pos.x.clamp(min.x, max.x), pos.y.clamp(min.y, max.y));
    pos.distance(closest) < radius
}

fn in_sight(player: Vec2, target: Vec2, segments: &[Segment]) -> bool {
    let to_target = target - player;
    let distance = to_target.length();
    if distance > LIGHT_RADIUS {
        return false;
    }

    let direction = to_target.normalize_or_zero();
    !segments.iter().any(|segment| {
        ray_segment_intersection(player, direction, *segment).is_some_and(|(_, t)| t < distance)
    })
}

fn visibility_polygon(origin: Vec2, radius: f32, segments: &[Segment]) -> Vec<Vec2> {
    let mut angles = Vec::new();
    for segment in segments {
        for point in [segment.a, segment.b] {
            let angle = (point - origin).to_angle();
            angles.extend([angle - 0.0006, angle, angle + 0.0006]);
        }
    }

    let mut hits = Vec::new();
    for angle in angles {
        let direction = Vec2::from_angle(angle);
        let mut closest = origin + direction * radius;
        let mut closest_t = radius;

        for segment in segments {
            if let Some((hit, t)) = ray_segment_intersection(origin, direction, *segment) {
                if t < closest_t {
                    closest = hit;
                    closest_t = t;
                }
            }
        }

        hits.push((angle, closest));
    }

    hits.sort_by(|a, b| a.0.total_cmp(&b.0));
    hits.into_iter().map(|(_, point)| point).collect()
}

fn ray_segment_intersection(
    origin: Vec2,
    direction: Vec2,
    segment: Segment,
) -> Option<(Vec2, f32)> {
    let s = segment.b - segment.a;
    let denominator = direction.perp_dot(s);
    if denominator.abs() < 0.0001 {
        return None;
    }

    let q = segment.a - origin;
    let t = q.perp_dot(s) / denominator;
    let u = q.perp_dot(direction) / denominator;

    if t >= 0.0 && (0.0..=1.0).contains(&u) {
        Some((origin + direction * t, t))
    } else {
        None
    }
}

fn wall_segments(walls: &Query<(&Transform, &Wall)>) -> Vec<Segment> {
    let mut segments = vec![
        Segment::new(
            Vec2::new(-WIDTH * 0.5, -HEIGHT * 0.5),
            Vec2::new(WIDTH * 0.5, -HEIGHT * 0.5),
        ),
        Segment::new(
            Vec2::new(WIDTH * 0.5, -HEIGHT * 0.5),
            Vec2::new(WIDTH * 0.5, HEIGHT * 0.5),
        ),
        Segment::new(
            Vec2::new(WIDTH * 0.5, HEIGHT * 0.5),
            Vec2::new(-WIDTH * 0.5, HEIGHT * 0.5),
        ),
        Segment::new(
            Vec2::new(-WIDTH * 0.5, HEIGHT * 0.5),
            Vec2::new(-WIDTH * 0.5, -HEIGHT * 0.5),
        ),
    ];

    for (transform, wall) in walls {
        let centre = transform.translation.truncate();
        let min = centre - wall.half;
        let max = centre + wall.half;
        segments.extend([
            Segment::new(Vec2::new(min.x, min.y), Vec2::new(max.x, min.y)),
            Segment::new(Vec2::new(max.x, min.y), Vec2::new(max.x, max.y)),
            Segment::new(Vec2::new(max.x, max.y), Vec2::new(min.x, max.y)),
            Segment::new(Vec2::new(min.x, max.y), Vec2::new(min.x, min.y)),
        ]);
    }

    segments
}

fn darkness_mesh(light_polygon: &[Vec2]) -> Mesh {
    let mut positions = vec![
        [-WIDTH * 0.5, -HEIGHT * 0.5, 0.0],
        [WIDTH * 0.5, -HEIGHT * 0.5, 0.0],
        [WIDTH * 0.5, HEIGHT * 0.5, 0.0],
        [-WIDTH * 0.5, HEIGHT * 0.5, 0.0],
    ];
    let mut indices = vec![0_u32, 1, 2, 0, 2, 3];

    let base = positions.len() as u32;
    for point in light_polygon.iter().rev() {
        positions.push([point.x, point.y, 0.0]);
    }

    if light_polygon.len() >= 3 {
        for i in 1..light_polygon.len() - 1 {
            indices.extend([base, base + i as u32, base + i as u32 + 1]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn cursor_world(
    windows: &Query<&Window>,
    camera: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    let cursor = windows.single().ok()?.cursor_position()?;
    let (camera, transform) = camera.single().ok()?;
    camera.viewport_to_world_2d(transform, cursor).ok()
}

fn level_walls() -> [Rect; 6] {
    [
        Rect::new(-210.0, 185.0, 220.0, 28.0),
        Rect::new(230.0, 195.0, 180.0, 28.0),
        Rect::new(-240.0, -115.0, 220.0, 28.0),
        Rect::new(250.0, -139.0, 240.0, 28.0),
        Rect::new(-24.0, 5.0, 32.0, 190.0),
        Rect::new(57.0, -10.0, 32.0, 140.0),
    ]
}

impl Rect {
    fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            centre: Vec2::new(x, y),
            half: Vec2::new(width * 0.5, height * 0.5),
        }
    }
}

impl Segment {
    fn new(a: Vec2, b: Vec2) -> Self {
        Self { a, b }
    }
}

fn random_range(min: f32, max: f32) -> f32 {
    min + (max - min) * rand_unit()
}

fn rand_index(max: usize) -> usize {
    (rand_unit() * max as f32).floor() as usize
}

fn rand_unit() -> f32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEED: AtomicU32 = AtomicU32::new(0x1234abcd);
    let old = SEED.load(Ordering::Relaxed);
    let new = old.wrapping_mul(1664525).wrapping_add(1013904223);
    SEED.store(new, Ordering::Relaxed);
    new as f32 / u32::MAX as f32
}
