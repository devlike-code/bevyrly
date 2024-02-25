pub mod anim;
pub mod gamepad;
pub mod geometry;

use std::{
    f32::consts::{FRAC_PI_2, FRAC_PI_4},
    marker::PhantomData,
    time::Duration,
};

use anim::{animate_sprites, AnimationIndices, AnimationTimer};
use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest},
    log::{Level, LogPlugin},
    prelude::*,
    render::view::RenderLayers,
};
use bevy_asset_loader::{
    asset_collection::AssetCollection,
    loading_state::{LoadingState, LoadingStateAppExt},
    standard_dynamic_asset::StandardDynamicAssetCollection,
};
use bevy_common_assets::ron::RonAssetPlugin;
use bevy_mod_imgui::{ImguiContext, ImguiPlugin};
use bevy_spatial::{kdtree::KDTree2, AutomaticUpdate, SpatialAccess, SpatialStructure};
use bevy_trauma_shake::{Shake, TraumaPlugin};
use gamepad::{gamepad_connections, gamepad_input, PlayerInputEvent};
use geometry::Line;
use lerp::Lerp;
use noisy_bevy::simplex_noise_2d;
use rand::{rngs::ThreadRng, Rng};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Debug, Default, States)]
enum GameStates {
    #[default]
    AssetLoading,
    LevelLoading,
    Gameplay,
}

macro_rules! destroy_entity {
    ($c:ident, $e:ident) => {
        let _ = $c.get_entity($e).map(|e| e.despawn_recursive());
    };
}

#[derive(AssetCollection, Resource)]
pub struct ImageAssets {
    #[asset(key = "small_ships")]
    pub small_ships: Handle<TextureAtlas>,

    #[asset(key = "large_ships")]
    pub large_ships: Handle<TextureAtlas>,

    #[asset(key = "smoke")]
    pub smoke: Handle<TextureAtlas>,

    #[asset(key = "hp_bar_empty")]
    pub hp_bar_empty: Handle<TextureAtlas>,

    #[asset(key = "hp_bar_full")]
    pub hp_bar_full: Handle<TextureAtlas>,

    #[asset(key = "explosion")]
    pub explosion: Handle<TextureAtlas>,

    #[asset(key = "hp_box")]
    pub hp_box: Handle<TextureAtlas>,

    #[asset(key = "dialogue")]
    pub dialogue: Handle<TextureAtlas>,

    #[asset(key = "debris")]
    pub debris: Handle<TextureAtlas>,
}

#[derive(Component, Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Ship {
    SmallShip(u32),
    LargeShip(u32),
}

impl Ship {
    pub fn get_frame(&self) -> u32 {
        match self {
            Ship::SmallShip(n) => *n,
            Ship::LargeShip(n) => *n,
        }
    }

    pub fn get_atlas(&self, image: &Res<ImageAssets>) -> Handle<TextureAtlas> {
        match self {
            Ship::SmallShip(_) => image.small_ships.clone(),
            Ship::LargeShip(_) => image.large_ships.clone(),
        }
    }
}

#[derive(Component)]
pub struct Name(String);

#[derive(Component)]
pub struct Player;

#[derive(Resource)]
pub struct PlayerSettings {
    scan_radius: f32,
    railgun_cooldown: f32,
    railgun_range: f32,
    missile_cooldown: f32,
    missile_lifetime: f32,
    missile_count: i32,
    missile_angle: f32,
    camera_speed: f32,
    camera_offset: f32,
    camera_deadzone: f32,
    show_gizmos: bool,
    show_debug: bool,
    use_rumble: bool,
    time_between_rumbles: f32,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            scan_radius: 300.0,
            railgun_cooldown: 0.03,
            railgun_range: 10.0,
            missile_cooldown: 0.1,
            missile_lifetime: 0.01,
            missile_count: 10,
            missile_angle: 1.0,
            camera_speed: 0.05,
            camera_offset: 100.0,
            camera_deadzone: 150.0,
            show_gizmos: false,
            show_debug: false,
            use_rumble: true,
            time_between_rumbles: 0.1,
        }
    }
}

#[derive(Component)]
pub struct GameObject;

#[derive(Component)]
pub struct TurnSpeed(f32);

#[derive(Component)]
pub struct MoveSpeed(f32);

#[derive(Component)]
pub struct StrafeSpeed(f32);

#[derive(Component)]
pub struct Angle(f32);

#[derive(Component)]
pub struct Thrust(f32);

#[derive(Deserialize, Serialize, TypePath, Asset, Debug)]
pub struct ShipBlueprint {
    name: String,
    ship: Ship,
    health: u32,
    turn_speed: f32,
    move_speed: f32,
    player: bool,
}

#[derive(Deserialize, Serialize, TypePath, Asset, Debug)]
pub struct LevelBlueprint {
    ships: Vec<ShipBlueprint>,
}

#[derive(Event)]
pub struct ThrustEvent {
    pub entity: Entity,
    pub thrust: f32,
    pub side: i8,
}

#[derive(Component, Default)]
pub struct HpBar;

#[derive(Component)]
pub struct HpBarContent;

#[derive(Component)]
pub struct Health(u32, u32);

#[derive(Component)]
pub struct GameCamera;

#[derive(Component)]
pub struct GameCameraTarget(pub Vec3);

#[derive(Component)]
pub struct UICamera;

#[derive(Component)]
pub struct UiPosition(pub Vec2);

#[derive(Component, Default)]
pub struct Dialogue;

#[derive(Event)]
pub struct DamageEvent(pub Entity, pub u32);

#[derive(Event)]
pub struct ToggleUI<T: Component>(pub Option<bool>, pub(crate) PhantomData<T>);

impl<T: Component> ToggleUI<T> {
    pub fn show() -> Self {
        Self(Some(true), Default::default())
    }

    pub fn hide() -> Self {
        Self(Some(false), Default::default())
    }
}

impl<T: Component> Default for ToggleUI<T> {
    fn default() -> Self {
        Self(None, Default::default())
    }
}

#[derive(Resource)]
struct LevelHandle(Handle<LevelBlueprint>);

#[derive(Resource, Default)]
pub struct Ships(pub Vec<ShipBlueprint>);

#[derive(Component)]
pub struct FireTarget(pub bool);

#[derive(Component)]
pub struct SpatialElement(pub f32);

#[derive(Component, Default)]
pub struct PDCSlug;

#[derive(Component, Default)]
pub struct Rail;

#[derive(Component)]
pub struct Fadeout(pub f32);

#[derive(Component)]
pub struct Missile;

#[derive(Resource, Default)]
pub struct MissileCooldown(pub f32);

#[derive(Component, PartialEq, Eq)]
pub enum Side {
    Player,
    Enemy,
}

pub trait Gun {
    type Bullet: Component + Default;
}

pub struct PDCTurret;
impl Gun for PDCTurret {
    type Bullet = PDCSlug;
}

#[derive(Component, Default)]
pub struct BulletPod<T: Gun> {
    pub heat: f32,
    pub range: f32,
    _phantom: PhantomData<T>,
}

impl<T: Gun> BulletPod<T> {
    pub fn new(heat: f32, range: f32) -> Self {
        Self {
            heat,
            range,
            _phantom: PhantomData,
        }
    }
}

#[derive(Component)]
pub struct MissileTarget(pub Entity);

#[derive(Component)]
pub struct ActivationTime(pub f32);

#[derive(Component)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct Noise;

#[derive(Event)]
pub struct FireMissileEvent(pub Entity);

fn smooth_function(x: f32, k: f32) -> f32 {
    1.0 / (1.0 + (-k * (x - 0.5)).exp())
}

#[derive(Event, Default)]
pub enum SpawnVisualEvent {
    #[default]
    None,
    Smoke {
        origin: Vec2,
        rotation: f32,
        scale: f32,
    },
    Explosion(Vec2),
    Debris(Vec2),
}

impl SpawnVisualEvent {
    pub fn default_smoke(origin: Vec2) -> SpawnVisualEvent {
        SpawnVisualEvent::Smoke {
            origin,
            rotation: 0.0,
            scale: 1.0,
        }
    }
}

fn spawn_smoke(
    commands: &mut Commands,
    image_assets: &Res<ImageAssets>,
    origin: &Transform,
    offset: Vec2,
    rotation: f32,
    scale: f32,
) {
    commands.spawn((
        GameObject,
        SpatialElement(10.0),
        SpriteSheetBundle {
            transform: Transform {
                translation: origin.translation + Vec3::new(offset.x, offset.y, 0.0),
                rotation: origin.rotation * Quat::from_axis_angle(Vec3::new(0., 0., 1.), rotation),
                scale: Vec3::ONE * scale,
            },
            sprite: TextureAtlasSprite::new(0),
            texture_atlas: image_assets.smoke.clone(),
            ..Default::default()
        },
        AnimationIndices { first: 0, last: 5 },
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Once)),
    ));
}

fn spawn_explosion(
    commands: &mut Commands,
    image_assets: &Res<ImageAssets>,
    origin: &Transform,
    offset: Vec2,
    rotation: f32,
    scale: f32,
) {
    let mut rng = rand::thread_rng();
    commands.spawn((
        GameObject,
        SpatialElement(10.0),
        SpriteSheetBundle {
            transform: Transform {
                translation: origin.translation + Vec3::new(offset.x, offset.y, 0.0),
                rotation: origin.rotation * Quat::from_axis_angle(Vec3::new(0., 0., 1.), rotation),
                scale: Vec3::ONE * scale,
            },
            sprite: TextureAtlasSprite::new(0),
            texture_atlas: image_assets.explosion.clone(),
            ..Default::default()
        },
        AnimationIndices {
            first: rng.gen_range(0..3),
            last: rng.gen_range(7..=10),
        },
        AnimationTimer(Timer::from_seconds(0.02, TimerMode::Once)),
    ));
}

fn spawn_debris(
    commands: &mut Commands,
    image_assets: &Res<ImageAssets>,
    origin: &Transform,
    size: f32,
) {
    commands.spawn((
        GameObject,
        SpatialElement(20.0),
        SpriteSheetBundle {
            transform: Transform {
                translation: origin.translation - Vec3::new(0., 0., 0.1),
                scale: Vec3::ONE * 2.0 * size,
                ..Default::default()
            },
            sprite: TextureAtlasSprite::new(0),
            texture_atlas: image_assets.debris.clone(),
            ..Default::default()
        },
    ));
}

type Space = KDTree2<SpatialElement>;

fn main() {
    App::new()
        .add_event::<SpawnVisualEvent>()
        .add_event::<PlayerInputEvent>()
        .add_event::<ThrustEvent>()
        .add_event::<FireMissileEvent>()
        .add_event::<DamageEvent>()
        .add_event::<ToggleUI<HpBar>>()
        .add_event::<ToggleUI<Dialogue>>()
        .init_resource::<PlayerSettings>()
        .init_resource::<Ships>()
        .init_resource::<MissileCooldown>()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(LogPlugin {
                    level: Level::ERROR,
                    ..Default::default()
                }),
        )
        .add_plugins(
            AutomaticUpdate::<SpatialElement>::new()
                .with_spatial_ds(SpatialStructure::KDTree2)
                .with_frequency(Duration::from_millis(5)),
        )
        .add_plugins(ImguiPlugin::default())
        .add_plugins(TraumaPlugin)
        .add_plugins(RonAssetPlugin::<LevelBlueprint>::new(&["level.ron"]))
        .add_systems(Update, gamepad_connections)
        .add_state::<GameStates>()
        .add_loading_state(
            LoadingState::new(GameStates::AssetLoading).continue_to_state(GameStates::LevelLoading),
        )
        .add_collection_to_loading_state::<_, ImageAssets>(GameStates::AssetLoading)
        .add_dynamic_collection_to_loading_state::<_, StandardDynamicAssetCollection>(
            GameStates::AssetLoading,
            "maz.assets.ron",
        )
        .add_systems(OnEnter(GameStates::LevelLoading), load_resources)
        .add_systems(
            Update,
            wait_for_level_resources
                .run_if(in_state(GameStates::LevelLoading))
                .run_if(on_event::<AssetEvent<LevelBlueprint>>()),
        )
        .add_systems(OnEnter(GameStates::Gameplay), (spawn_level, spawn_ui))
        .add_systems(
            PostUpdate,
            (
                (
                    consume_spawn_visual_events,
                    render_screenspace_ui,
                    render_player_health_ui,
                )
                    .chain()
                    .run_if(in_state(GameStates::Gameplay)),
                camera_follow,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (thrust_emits_smoke
                .run_if(in_state(GameStates::Gameplay))
                .run_if(on_event::<ThrustEvent>()),),
        )
        .add_systems(
            Update,
            player_fire_missiles
                .run_if(in_state(GameStates::Gameplay))
                .run_if(on_event::<FireMissileEvent>()),
        )
        .add_systems(
            Update,
            player_missile_cooldown.run_if(in_state(GameStates::Gameplay)),
        )
        .add_systems(Update, spawn_enemies.run_if(in_state(GameStates::Gameplay)))
        .add_systems(Update, fire_pdc.run_if(in_state(GameStates::Gameplay)))
        .add_systems(Update, fire_railguns.run_if(in_state(GameStates::Gameplay)))
        .add_systems(
            Update,
            (missile_guidance, fly_velocity)
                .chain()
                .run_if(in_state(GameStates::Gameplay)),
        )
        .add_systems(
            PostUpdate,
            (
                animate_sprites,
                rail_collisions,
                pdc_collisions,
                missile_explode_against_ship,
                fadeout,
                destroy_when_health_reaches_zero,
            )
                .chain()
                .run_if(in_state(GameStates::Gameplay)),
        )
        .add_systems(
            Update,
            (
                gamepad_input.run_if(in_state(GameStates::Gameplay)),
                control_ship.run_if(in_state(GameStates::Gameplay)),
                debug_input.run_if(in_state(GameStates::Gameplay)),
                scan_surroundings.run_if(in_state(GameStates::Gameplay)),
                resolve_damage.run_if(in_state(GameStates::Gameplay)),
                shake_on_player_damage.run_if(on_event::<DamageEvent>()),
                show_ui_on_damage.run_if(on_event::<DamageEvent>()),
                show_ui_elements::<HpBar>.run_if(on_event::<ToggleUI<HpBar>>()),
                show_ui_elements::<Dialogue>.run_if(on_event::<ToggleUI<Dialogue>>()),
            )
                .chain(),
        )
        .add_systems(
            OnExit(GameStates::Gameplay),
            (
                cleanup_entities::<GameObject>,
                cleanup_resources::<LevelHandle>,
            ),
        )
        .add_systems(Update, (show_debug_window, debug_show_targets))
        .run();
}

fn load_resources(mut commands: Commands, asset_server: Res<AssetServer>) {
    let level = LevelHandle(asset_server.load("level1.level.ron"));
    commands.insert_resource(level);
}

fn wait_for_level_resources(mut game_state: ResMut<NextState<GameStates>>) {
    game_state.set(GameStates::Gameplay);
}

fn cleanup_entities<T: Component>(mut commands: Commands, query_t: Query<Entity, With<T>>) {
    for e in query_t.iter() {
        destroy_entity!(commands, e);
    }
}

fn cleanup_resources<R: Resource>(mut commands: Commands) {
    commands.remove_resource::<R>();
}

fn spawn_enemies(
    mut commands: Commands,
    mut wait_time: Local<f32>,
    image_assets: Res<ImageAssets>,
    time: Res<Time>,
    ships: Res<Ships>,
) {
    *wait_time += time.delta_seconds();
    if *wait_time < 10.0 {
        return;
    }

    let mut rng = rand::thread_rng();

    let Some(blueprint) = ships.0.get(rng.gen_range(0..ships.0.len())) else {
        return;
    };

    spawn_ship(
        &mut commands,
        &image_assets,
        blueprint,
        Vec2::new(rng.gen_range(-400.0..400.0), rng.gen_range(-400.0..400.0)),
    );

    *wait_time = 0.0;
}

fn spawn_ship(
    commands: &mut Commands,
    image_assets: &Res<ImageAssets>,
    ship_blueprint: &ShipBlueprint,
    position: Vec2,
) {
    let mut e = commands.spawn((
        Name(ship_blueprint.name.clone()),
        SpatialElement(10.0),
        TurnSpeed(ship_blueprint.turn_speed),
        MoveSpeed(ship_blueprint.move_speed),
        StrafeSpeed(0.0),
        Angle(0.0),
        Thrust(0.0),
        GameObject,
        Health(ship_blueprint.health, ship_blueprint.health),
        ship_blueprint.ship,
        SpriteSheetBundle {
            transform: Transform {
                translation: Vec3::new(position.x, position.y, 0.0),
                ..Default::default()
            },
            sprite: TextureAtlasSprite::new(ship_blueprint.ship.get_frame() as usize),
            texture_atlas: ship_blueprint.ship.get_atlas(image_assets),
            ..Default::default()
        },
    ));

    if ship_blueprint.player {
        e.insert((Player, Side::Player));
    } else {
        e.insert((
            Side::Enemy,
            FireTarget(false),
            BulletPod::<PDCTurret>::new(-10.0, 250.0),
        ));
    }
}

fn spawn_level(
    mut commands: Commands,
    mut ships: ResMut<Ships>,
    level: Res<LevelHandle>,
    asset_server: Res<AssetServer>,
    image_assets: Res<ImageAssets>,
    mut levels: ResMut<Assets<LevelBlueprint>>,
) {
    commands
        .spawn((
            Name("dialogue".into()),
            Dialogue,
            GameObject,
            UiPosition(Vec2::new(200.0, 200.0)),
            SpriteSheetBundle {
                transform: Transform {
                    scale: Vec3::ONE * 2.0,
                    ..Default::default()
                },
                visibility: Visibility::Hidden,
                sprite: TextureAtlasSprite::new(0),
                texture_atlas: image_assets.dialogue.clone(),
                ..Default::default()
            },
            RenderLayers::layer(1),
        ))
        .with_children(|parent| {
            let font = asset_server.load("fonts/NeuePixelSans.ttf");
            let text_style = TextStyle {
                font: font.clone(),
                font_size: 20.0,
                color: Color::WHITE,
            };

            parent.spawn((
                Text2dBundle {
                    text: Text::from_section("XAN: Prepare to die, human!", text_style.clone())
                        .with_alignment(TextAlignment::Left),
                    transform: Transform {
                        translation: Vec3::new(-40.0, 10.0, 10.0),
                        scale: Vec3::ONE * 0.5,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                RenderLayers::layer(1),
            ));
        });

    commands.spawn((
        Name("game camera".into()),
        GameObject,
        GameCamera,
        GameCameraTarget(Vec3::ZERO),
        Camera2dBundle {
            camera: Camera {
                order: 0,
                ..Default::default()
            },
            ..Default::default()
        },
        Shake::default(),
        RenderLayers::layer(0),
    ));

    commands.spawn((
        Name("ui camera".into()),
        GameObject,
        UICamera,
        Camera2dBundle {
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::None,
            },
            camera: Camera {
                order: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        Shake::default(),
        RenderLayers::layer(1),
    ));

    if let Some(level) = levels.remove(level.0.id()) {
        for ship_blueprint in level.ships {
            if ship_blueprint.player {
                spawn_ship(&mut commands, &image_assets, &ship_blueprint, Vec2::ZERO);
            } else {
                ships.0.push(ship_blueprint);
            }
        }
    } else {
        println!("Level failed to load.");
    }
}

fn consume_spawn_visual_events(
    mut commands: Commands,
    image_assets: Res<ImageAssets>,
    mut spawn_visual: EventReader<SpawnVisualEvent>,
) {
    for se in spawn_visual.read() {
        match se {
            SpawnVisualEvent::None => {}
            SpawnVisualEvent::Smoke {
                origin,
                rotation,
                scale,
            } => spawn_smoke(
                &mut commands,
                &image_assets,
                &Transform {
                    translation: Vec3::new(origin.x, origin.y, 0.0),
                    ..Default::default()
                },
                Vec2::ZERO,
                *rotation,
                *scale,
            ),
            SpawnVisualEvent::Explosion(pos) => spawn_explosion(
                &mut commands,
                &image_assets,
                &Transform {
                    translation: Vec3::new(pos.x, pos.y, 0.0),
                    ..Default::default()
                },
                Vec2::ZERO,
                0.0,
                1.0,
            ),
            SpawnVisualEvent::Debris(pos) => spawn_debris(
                &mut commands,
                &image_assets,
                &Transform {
                    translation: Vec3::new(pos.x, pos.y, 0.0),
                    ..Default::default()
                },
                1.0,
            ),
        }
    }
}

fn destroy_when_health_reaches_zero(
    mut commands: Commands,
    mut spawn_visual: EventWriter<SpawnVisualEvent>,
    health_query: Query<(Entity, &Health, &Transform)>,
) {
    //
    for (e, health, transform) in &health_query {
        if health.1 == 0 {
            destroy_entity!(commands, e);
            spawn_visual.send(SpawnVisualEvent::default_smoke(transform.translation.xy()));
            spawn_visual.send(SpawnVisualEvent::Debris(transform.translation.xy()));
        }
    }
}

fn spawn_ui(mut commands: Commands, image_assets: Res<ImageAssets>) {
    commands
        .spawn((
            GameObject,
            UiPosition(Vec2::new(190.0, 15.0)),
            Name("hp".into()),
            HpBar,
            SpriteSheetBundle {
                transform: Transform {
                    translation: Vec3::new(0., 0., 100.0),
                    scale: Vec3::ONE * 1.5,
                    ..Default::default()
                },
                visibility: Visibility::Hidden,
                sprite: TextureAtlasSprite::new(0),
                texture_atlas: image_assets.hp_bar_empty.clone(),
                ..Default::default()
            },
            RenderLayers::layer(1),
        ))
        .with_children(|parent| {
            (0..36).for_each(|i| {
                parent.spawn((
                    HpBarContent,
                    GameObject,
                    Name(format!("hp-box-{}", i)),
                    SpriteSheetBundle {
                        transform: Transform {
                            translation: Vec3::new(i as f32 * 4. - 85.0, 0., 100.0),
                            ..Default::default()
                        },
                        sprite: TextureAtlasSprite::new(0),
                        texture_atlas: image_assets.hp_box.clone(),
                        ..Default::default()
                    },
                    RenderLayers::layer(1),
                ));
            });
        });
}

fn render_player_health_ui(
    children: Query<&mut Children>,
    mut hp: Query<Entity, With<HpBar>>,
    mut vis: Query<&mut Visibility>,
    player_health_query: Query<&Health, With<Player>>,
) {
    if let Ok(entity) = hp.get_single_mut() {
        let Ok(health) = player_health_query.get_single() else {
            return;
        };

        let parent_visible = vis.get(entity).unwrap() == Visibility::Visible;
        let children = children
            .get(entity)
            .unwrap()
            .iter()
            .enumerate()
            .collect::<Vec<_>>();
        for (i, child) in children {
            let _ = vis.get_mut(*child).map(|mut v| {
                *v = if health.1 > i as u32 && parent_visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                }
            });
        }
    }
}

fn render_screenspace_ui(
    camera: Query<(&Camera, &GlobalTransform), With<UICamera>>,
    mut ui_elements: Query<(&mut Transform, &UiPosition)>,
) {
    for (mut transform, pos) in &mut ui_elements {
        if let Ok((camera, camera_transform)) = camera.get_single() {
            let Some(point) = camera.viewport_to_world_2d(camera_transform, pos.0) else {
                return;
            };

            transform.translation.x = point.x;
            transform.translation.y = point.y;
        }
    }
}

fn debug_input(
    input: Res<Input<KeyCode>>,
    mut next_state: ResMut<NextState<GameStates>>,
    mut player: Query<Entity, With<Player>>,
    mut player_settings: ResMut<PlayerSettings>,
    mut damage_events: EventWriter<DamageEvent>,
    mut toggle_ui: EventWriter<ToggleUI<HpBar>>,
) {
    if input.just_pressed(KeyCode::F1) {
        player_settings.show_debug = !player_settings.show_debug;
    }

    if input.just_pressed(KeyCode::F2) {
        toggle_ui.send(ToggleUI::<HpBar>::default());
    }

    if input.just_pressed(KeyCode::F3) {
        player_settings.use_rumble = !player_settings.use_rumble;
    }

    if input.just_pressed(KeyCode::F4) {
        player_settings.show_gizmos = !player_settings.show_gizmos;
    }

    if input.just_pressed(KeyCode::F5) {
        next_state.set(GameStates::AssetLoading);
    }

    if input.just_pressed(KeyCode::D) {
        if let Ok(e) = player.get_single_mut() {
            damage_events.send(DamageEvent(e, rand::thread_rng().gen_range(1..10)));
        }
    }
}

#[allow(clippy::type_complexity)]
fn scan_surroundings(
    mut gizmos: Gizmos,
    player_settings: Res<PlayerSettings>,
    mut player_query: Query<&Transform, With<Player>>,
    ship_query: Query<(Entity, &Transform), (Without<Player>, With<Ship>)>,
    mut targets: Query<&mut FireTarget>,
) {
    let Ok(player_transform) = player_query.get_single_mut() else {
        return;
    };

    let movement_direction = player_transform.rotation * Vec3::Y;
    let scan_radius = player_settings.scan_radius;

    let line = Line(
        player_transform.translation.xy(),
        player_transform.translation.xy() + movement_direction.xy() * scan_radius,
    );

    if player_settings.show_gizmos {
        gizmos.line_2d(line.0, line.1, Color::WHITE);
    }

    let l1 = line.1 - line.0;

    for (entity, ship_transform) in &ship_query {
        let dist = player_transform
            .translation
            .xy()
            .distance(ship_transform.translation.xy());
        let l2 = ship_transform.translation.xy() - line.0;

        let a = l1.angle_between(l2);
        if a.abs() <= FRAC_PI_4 && dist < player_settings.scan_radius {
            if let Ok(mut target) = targets.get_mut(entity) {
                target.0 = true;
            }
        } else if let Ok(mut target) = targets.get_mut(entity) {
            target.0 = false;
        }
    }
}

#[allow(clippy::type_complexity)]
fn debug_show_targets(
    mut gizmos: Gizmos,
    ship_query: Query<(&Transform, &FireTarget), (Without<Player>, With<Ship>)>,
) {
    for (transform, FireTarget(marked)) in &ship_query {
        if *marked {
            gizmos.circle_2d(transform.translation.xy(), 30.0, Color::WHITE);
        }
    }
}

#[allow(clippy::type_complexity)]
fn control_ship(
    mut input: EventReader<PlayerInputEvent>,
    mut player_query: Query<
        (
            Entity,
            &mut Transform,
            &mut Thrust,
            &mut StrafeSpeed,
            &TurnSpeed,
            &MoveSpeed,
        ),
        With<Player>,
    >,
    mut thrust_events: EventWriter<ThrustEvent>,
    mut fire_events: EventWriter<FireMissileEvent>,
) {
    let Ok((entity, mut player_transform, mut thrust, mut strafe_speed, turn_speed, move_speed)) =
        player_query.get_single_mut()
    else {
        return;
    };

    let mut throttle = false;
    for ev in input.read() {
        if let Some(xy @ Vec2 { x, y }) = ev.xy {
            let target_angle = (-y).atan2(-x) + FRAC_PI_4;
            let quat = Quat::from_axis_angle(Vec3::new(0., 0., 1.), target_angle);
            player_transform.rotation = player_transform
                .rotation
                .slerp(quat, (turn_speed.0 * thrust.0).max(0.02));

            if xy.length() > 0.1 {
                throttle = true;
                thrust.0 = thrust.0.lerp(1.0, 0.1);
            }
        }

        let mut movement_direction = player_transform.rotation * Vec3::Y;
        let right = Vec3::new(movement_direction.y, -movement_direction.x, 0.0);

        if strafe_speed.0 == 0.0 {
            if ev.keys.contains(&GamepadButtonType::LeftTrigger) {
                strafe_speed.0 = -3.0;
                thrust_events.send(ThrustEvent {
                    entity,
                    thrust: 1.0,
                    side: -1,
                });
            }

            if ev.keys.contains(&GamepadButtonType::RightTrigger) {
                strafe_speed.0 = 3.0;
                thrust_events.send(ThrustEvent {
                    entity,
                    thrust: 1.0,
                    side: 1,
                });
            }

            if ev.keys.contains(&GamepadButtonType::LeftTrigger2) {
                fire_events.send(FireMissileEvent(entity));
            }

            if ev.keys.contains(&GamepadButtonType::RightTrigger2) {
                fire_events.send(FireMissileEvent(entity));
            }
        } else {
            #[allow(clippy::collapsible_else_if)]
            if strafe_speed.0 > 0.0 {
                strafe_speed.0 *= 0.895;
                if strafe_speed.0 < 0.1 {
                    strafe_speed.0 = 0.0
                }
            } else if strafe_speed.0 < 0.0 {
                strafe_speed.0 *= 0.895;
                if strafe_speed.0 > -0.1 {
                    strafe_speed.0 = 0.0
                }
            }
        }

        movement_direction += right * strafe_speed.0;

        player_transform.translation.x += movement_direction.x * thrust.0 * move_speed.0;
        player_transform.translation.y += movement_direction.y * thrust.0 * move_speed.0;

        if thrust.0 > 0.0 {
            thrust_events.send(ThrustEvent {
                entity,
                thrust: thrust.0,
                side: 0,
            });
        }
    }

    if !throttle {
        thrust.0 = thrust.0.lerp(0.0, 0.015);
    }
}

pub fn missile_explode_against_ship(
    mut commands: Commands,
    mut visual_events: EventWriter<SpawnVisualEvent>,
    mut damage_events: EventWriter<DamageEvent>,
    missile_query: Query<(Entity, &Transform, &Side), With<Missile>>,
    ship_query: Query<&Side, With<Ship>>,
    space: Res<Space>,
) {
    for (entity, missile_transform, missile_side) in &missile_query {
        for (_, target) in space.within_distance(missile_transform.translation.xy(), 10.0) {
            if let Some(target) = target {
                if let Ok(target_side) = ship_query.get(target) {
                    if missile_side != target_side {
                        damage_events.send(DamageEvent(target, 1));
                        destroy_entity!(commands, entity);
                        visual_events.send(SpawnVisualEvent::Explosion(
                            missile_transform.translation.xy(),
                        ));
                    }
                }
            }
        }
    }
}

pub fn rail_collisions(
    mut commands: Commands,
    mut visual_events: EventWriter<SpawnVisualEvent>,
    player_settings: Res<PlayerSettings>,
    rail_query: Query<(Entity, &Transform), With<Rail>>,
    pdc_query: Query<Entity, With<PDCSlug>>,
    space: Res<Space>,
) {
    for (rail, transform) in &rail_query {
        let mut rail_destroyed = false;

        for (_, maybe_entity) in
            space.within_distance(transform.translation.xy(), player_settings.railgun_range)
        {
            let Some(entity) = maybe_entity else {
                continue;
            };

            if pdc_query.contains(entity) {
                visual_events.send(SpawnVisualEvent::default_smoke(transform.translation.xy()));
                //rail_destroyed = true;
                destroy_entity!(commands, entity);

                break;
            }
        }

        if rail_destroyed {
            destroy_entity!(commands, rail);
            continue;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn pdc_collisions(
    mut commands: Commands,
    mut visual_events: EventWriter<SpawnVisualEvent>,
    pdc_query: Query<(Entity, &Transform), With<PDCSlug>>,
    missile_query: Query<Entity, With<Missile>>,

    player_query: Query<Entity, With<Player>>,
    mut damage_events: EventWriter<DamageEvent>,
    space: Res<Space>,
) {
    let mut rng = rand::thread_rng();
    for (pdc, slug_transform) in &pdc_query {
        let mut pdc_destroyed = false;

        for (_, maybe_entity) in space.within_distance(slug_transform.translation.xy(), 2.0) {
            if let Some(entity) = maybe_entity {
                if missile_query.contains(entity) {
                    visual_events.send(SpawnVisualEvent::default_smoke(
                        slug_transform.translation.xy(),
                    ));

                    pdc_destroyed = true;
                    destroy_entity!(commands, entity);

                    break;
                } else if player_query.contains(entity) {
                    visual_events.send(SpawnVisualEvent::default_smoke(
                        slug_transform.translation.xy(),
                    ));
                    pdc_destroyed = rng.gen_bool(0.3);
                    if rng.gen_bool(0.5) {
                        damage_events.send(DamageEvent(entity, 1));
                    }
                }
            }
        }

        if pdc_destroyed {
            destroy_entity!(commands, pdc);
            continue;
        }
    }
}

pub fn fly_velocity(
    time: Res<Time>,
    mut commands: Commands,
    mut velocity_query: Query<(Entity, &mut Transform, &Velocity)>,
    mut activation_times: Query<&mut ActivationTime, With<Velocity>>,
    noise_query: Query<Entity, With<Noise>>,
) {
    for (entity, mut transform, velocity) in &mut velocity_query {
        let mut velocity_factor = 1.0;
        if let Ok(mut at) = activation_times.get_mut(entity) {
            at.0 -= time.delta_seconds();
            if at.0 < 0.0 {
                commands.entity(entity).remove::<ActivationTime>();
            }
            velocity_factor = 0.35;
        }

        let noise = if noise_query.contains(entity) {
            let right = Vec2::new(velocity.0.y, -velocity.0.x).normalize();
            right
                * simplex_noise_2d(transform.translation.xy())
                * velocity_factor
                * velocity_factor
                * transform.scale.x
                * transform.scale.x
        } else {
            Vec2::ZERO
        };

        let v = velocity.0 * velocity_factor + noise;
        transform.translation += Vec3::new(v.x, v.y, 0.0);
    }
}

fn missile_guidance(
    time: Res<Time>,
    mut missile_query: Query<
        (Entity, &mut Transform, &mut Velocity, &MissileTarget),
        With<Missile>,
    >,
    transform_query: Query<&Transform, Without<Missile>>,
    activation_times: Query<&ActivationTime>,
) {
    for (entity, mut missile_transform, mut velocity, MissileTarget(target)) in &mut missile_query {
        let Ok(target_transform) = transform_query.get(*target) else {
            continue;
        };

        if activation_times.contains(entity) {
            continue;
        }

        let target_position = target_transform.translation.xy();
        let missile_forward = (missile_transform.rotation * Vec3::Y).xy();

        let to_target = (target_position - missile_transform.translation.xy()).normalize();
        let forward_dot_target = missile_forward.dot(to_target);
        if (forward_dot_target - 1.0).abs() < f32::EPSILON {
            continue;
        }
        let missile_right = (missile_transform.rotation * Vec3::X).xy();
        let right_dot_target = missile_right.dot(to_target);
        let rotation_sign = -f32::copysign(1.0, right_dot_target);
        let max_angle = forward_dot_target.clamp(-1.0, 1.0).acos();
        let rotation_angle =
            rotation_sign * (f32::to_radians(270.0) * time.delta_seconds()).min(max_angle);
        missile_transform.rotate_z(rotation_angle);
        let velocity_len = velocity.0.length();
        let missile_forward = (missile_transform.rotation * Vec3::Y).xy().normalize();

        velocity.0.x = missile_forward.x * velocity_len;
        velocity.0.y = missile_forward.y * velocity_len;
    }
}

pub fn fadeout(
    mut commands: Commands,
    image_assets: Res<ImageAssets>,
    mut fadeout_query: Query<(Entity, &Fadeout, &Transform, &mut Sprite)>,
) {
    for (entity, fader, transform, mut sprite) in &mut fadeout_query {
        let alpha = sprite.color.a() - fader.0;
        if alpha > 0.0 {
            sprite.color.set_a(alpha);
        } else {
            sprite.color.set_a(0.0);
            spawn_explosion(
                &mut commands,
                &image_assets,
                transform,
                Vec2::ZERO,
                0.0,
                1.0,
            );
            destroy_entity!(commands, entity);
        }
    }
}

pub fn player_missile_cooldown(time: Res<Time>, mut cooldown: ResMut<MissileCooldown>) {
    //
    cooldown.0 -= time.delta_seconds();
}

#[allow(clippy::too_many_arguments)]
pub fn player_fire_missiles(
    mut commands: Commands,
    mut fire_events: EventReader<FireMissileEvent>,
    mut cooldown: ResMut<MissileCooldown>,
    player_settings: ResMut<PlayerSettings>,
    image_assets: Res<ImageAssets>,
    player_query: Query<&Transform, With<Player>>,
    fire_targets: Query<(Entity, &FireTarget)>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };

    if cooldown.0 > 0.0 {
        return;
    }

    let fire_targets = fire_targets
        .iter()
        .filter(|(_e, ft)| ft.0)
        .map(|(e, _ft)| e)
        .collect::<Vec<_>>();

    let mut rng = rand::thread_rng();

    let position = player_transform.translation;
    let direction = (player_transform.rotation * Vec3::Y).xy();
    let right = Vec3::new(direction.y, -direction.x, 0.0);

    for FireMissileEvent(_player) in fire_events.read() {
        for i in 0..player_settings.missile_count {
            let rotation = player_transform.rotation
                * Quat::from_axis_angle(
                    Vec3::new(0., 0., 1.),
                    i as f32 * 4.0 * 0.0174 * player_settings.missile_angle,
                );

            let missile = commands
                .spawn((
                    GameObject,
                    SpatialElement(3.0),
                    Sprite::default(),
                    SpriteSheetBundle {
                        transform: Transform {
                            translation: position + right * 5.0 + rng.gen_range(0.0..0.5),
                            rotation,
                            scale: Vec3::ONE,
                        },
                        sprite: TextureAtlasSprite::new(0),
                        texture_atlas: image_assets.hp_box.clone(),
                        ..Default::default()
                    },
                    Velocity((rotation * Vec3::Y).xy().normalize() * rng.gen_range(5.0..5.5)),
                    ActivationTime(rng.gen_range(0.5..0.95)),
                    Fadeout(player_settings.missile_lifetime),
                    Side::Player,
                    Missile,
                ))
                .id();

            if !fire_targets.is_empty() {
                commands.entity(missile).insert(MissileTarget(
                    fire_targets[rng.gen_range(0..fire_targets.len())],
                ));
            }
        }
    }
    cooldown.0 = player_settings.missile_cooldown;
}

#[allow(clippy::too_many_arguments)]
fn fire_artillery_at<G: Gun>(
    rng: &mut ThreadRng,
    pdc: &mut BulletPod<G>,
    image_assets: &Res<ImageAssets>,
    gizmos: &mut Gizmos,
    commands: &mut Commands,
    time: &Res<Time>,
    target_transform: &Transform,
    pdc_transform: &Transform,
    velocity: &Velocity,
    fadeout: f32,
    activation_time: f32,
    i: u32,
) {
    if pdc.heat > 0.0 {
        pdc.heat += time.delta_seconds();

        if pdc.heat > 5.0 {
            pdc.heat = -2.0;
        }

        let projected_target_position = target_transform.translation
            + Vec3::new(
                velocity.0.x + rng.gen_range(-2.0..2.0),
                velocity.0.y + rng.gen_range(-2.0..2.0),
                0.0,
            );

        gizmos.circle_2d(projected_target_position.xy(), 5.0, Color::RED);

        let direction = projected_target_position - pdc_transform.translation;

        let rotation = Quat::from_axis_angle(
            Vec3::new(0., 0., 1.),
            direction.y.atan2(direction.x) - FRAC_PI_2,
        );

        commands.spawn((
            GameObject,
            SpatialElement(1.0),
            Sprite::default(),
            SpriteSheetBundle {
                transform: Transform {
                    translation: pdc_transform.translation + rng.gen_range(0.0..0.5),
                    scale: Vec3::ONE * 0.33,
                    ..Default::default()
                },
                sprite: TextureAtlasSprite::new(0),
                texture_atlas: image_assets.hp_box.clone(),
                ..Default::default()
            },
            Velocity((rotation * Vec3::Y).xy().normalize() * 2.5),
            ActivationTime(activation_time),
            Fadeout(fadeout),
            Side::Enemy,
            G::Bullet::default(),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn fire_pdc(
    mut gizmos: Gizmos,
    time: Res<Time>,
    image_assets: Res<ImageAssets>,
    mut commands: Commands,
    missile_query: Query<(Entity, &Transform, &Velocity, &MissileTarget), With<Missile>>,
    player_query: Query<(Entity, &Transform), With<Player>>,
    transform_query: Query<&Transform>,
    mut pdc_query: Query<(&Transform, &mut BulletPod<PDCTurret>)>,
) {
    let mut rng = rand::thread_rng();

    for (pdc_transform, mut pdc) in &mut pdc_query {
        if pdc.heat < 0.0 {
            pdc.heat += time.delta_seconds();

            continue;
        }

        for (_, player_transform) in &player_query {
            if player_transform
                .translation
                .distance(pdc_transform.translation)
                > pdc.range
            {
                continue;
            }

            if rng.gen_bool(0.8) {
                continue;
            }

            for i in 0..10 {
                fire_artillery_at(
                    &mut rng,
                    &mut pdc,
                    &image_assets,
                    &mut gizmos,
                    &mut commands,
                    &time,
                    player_transform,
                    pdc_transform,
                    &Velocity(Vec2::ZERO),
                    0.005,
                    i as f32 * 0.05,
                    i,
                );
            }
        }
    }

    let mut i = 0;
    for (_missile_entity, missile_transform, velocity, target) in &missile_query {
        if let Ok((_, mut pdc)) = pdc_query.get_mut(target.0) {
            let Ok(ship_transform) = transform_query.get(target.0) else {
                continue;
            };

            fire_artillery_at(
                &mut rng,
                &mut pdc,
                &image_assets,
                &mut gizmos,
                &mut commands,
                &time,
                missile_transform,
                ship_transform,
                velocity,
                0.005,
                i as f32 * 0.05,
                i,
            );

            i += 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn fire_railguns(
    mut gizmos: Gizmos,
    mut wait_time: Local<f32>,
    player_settings: Res<PlayerSettings>,
    time: Res<Time>,
    image_assets: Res<ImageAssets>,
    mut commands: Commands,
    mut player_query: Query<(Entity, &Transform), With<Player>>,
    mut input: EventReader<PlayerInputEvent>,
) {
    let Ok((_player, player_transform)) = player_query.get_single_mut() else {
        return;
    };

    if *wait_time > 0.0 {
        *wait_time -= time.delta_seconds();
        return;
    }

    for ev in input.read() {
        if let Some(Vec2 { x, y }) = ev.dir {
            let translation = player_transform.translation + Vec3::new(x, y, 0.0) * 100.0;

            gizmos.circle_2d(translation.xy(), 5.0, Color::RED);

            let direction = translation - player_transform.translation;

            let rotation = Quat::from_axis_angle(
                Vec3::new(0., 0., 1.),
                direction.y.atan2(direction.x) - FRAC_PI_2,
            );

            commands.spawn((
                GameObject,
                SpatialElement(1.0),
                Sprite {
                    color: Color::RED,
                    ..Default::default()
                },
                SpriteSheetBundle {
                    transform: Transform {
                        translation: player_transform.translation,
                        scale: Vec3::ONE * 0.25,
                        ..Default::default()
                    },
                    sprite: TextureAtlasSprite::new(0),
                    texture_atlas: image_assets.hp_box.clone(),
                    ..Default::default()
                },
                Velocity((rotation * Vec3::Y).xy().normalize() * 4.0),
                Fadeout(0.025),
                Side::Player,
                Rail,
            ));
        }
    }

    *wait_time = player_settings.railgun_cooldown;
}

pub fn thrust_emits_smoke(
    mut visual_events: EventWriter<SpawnVisualEvent>,
    mut thrust_events: EventReader<ThrustEvent>,
    transform_query: Query<&Transform>,
) {
    let mut rng = rand::thread_rng();
    for thrust in thrust_events.read() {
        if thrust.side != 0 {
            if let Ok(transform) = transform_query.get(thrust.entity) {
                let forward = (transform.rotation * Vec3::Y).xy();
                let right = Vec2::new(forward.y, -forward.x);

                visual_events.send(SpawnVisualEvent::Smoke {
                    origin: transform.translation.xy()
                        + right * -thrust.side as f32 * 0.05
                        + forward * 20.0 * thrust.thrust,
                    rotation: FRAC_PI_4 * thrust.side as f32,
                    scale: 1.25 * rng.gen_range(0.5..1.25),
                });
            }
        } else if thrust.thrust > 0.5 && rng.gen::<f32>() < thrust.thrust * 0.5 {
            if let Ok(transform) = transform_query.get(thrust.entity) {
                let forward = (transform.rotation * Vec3::Y).xy();
                visual_events.send(SpawnVisualEvent::Smoke {
                    origin: transform.translation.xy()
                        + Vec2::new(
                            rng.gen_range(-1.0..1.0) * (1.0 - thrust.thrust) * 5.0,
                            rng.gen_range(-3.0..3.0),
                        )
                        - forward * 10.0,
                    rotation: 0.0,
                    scale: rng.gen_range(0.5..1.25),
                });
            }
        }
    }
}

pub fn show_ui_on_damage(
    mut damage_events: EventReader<DamageEvent>,
    mut show_ui: EventWriter<ToggleUI<HpBar>>,
    player: Query<&Player>,
) {
    for de in damage_events.read() {
        if player.contains(de.0) {
            show_ui.send(ToggleUI::<HpBar>::show());
        }
    }
}

pub fn show_ui_elements<T: Component>(
    mut toggle_ui: EventReader<ToggleUI<T>>,
    mut vis: Query<&mut Visibility, With<T>>,
) {
    for ToggleUI(state, ..) in toggle_ui.read() {
        for mut element in &mut vis {
            let next = match state {
                Some(true) => Visibility::Visible,
                Some(false) => Visibility::Hidden,
                None if *element == Visibility::Hidden => Visibility::Visible,
                None => Visibility::Hidden,
            };

            *element = next;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn shake_on_player_damage(
    mut damage_events: EventReader<DamageEvent>,
    mut rumble_requests: EventWriter<GamepadRumbleRequest>,
    mut shakes: Query<&mut Shake>,
    mut wait_time: Local<f32>,
    time: Res<Time>,
    gamepads: Res<Gamepads>,
    player_settings: Res<PlayerSettings>,
    player_query: Query<&Transform, With<Player>>,
    transform_query: Query<&Transform>,
) {
    if !player_settings.use_rumble {
        return;
    }

    let mut distance_from_player = f32::INFINITY;

    if time.elapsed_seconds() - *wait_time < player_settings.time_between_rumbles {
        *wait_time = time.elapsed_seconds();
        damage_events.clear();
        return;
    }

    let Ok(player_transform) = player_query.get_single() else {
        return;
    };

    for de in damage_events.read() {
        if let Ok(recipient_transform) = transform_query.get(de.0) {
            let distance = recipient_transform
                .translation
                .distance(player_transform.translation);

            if distance < distance_from_player {
                distance_from_player = distance;
            }
        }
    }

    damage_events.clear();

    let mut rumble_strength = distance_from_player / 100.0;
    let mut rumble = GamepadRumbleIntensity::MAX;
    if rumble_strength == 0.0 {
        rumble = GamepadRumbleIntensity::MAX;
    } else if rumble_strength < 0.3 {
        rumble = GamepadRumbleIntensity {
            strong_motor: 0.3,
            weak_motor: 0.5,
        };
    } else if rumble_strength < 1.0 {
        rumble = GamepadRumbleIntensity {
            strong_motor: 0.2,
            weak_motor: 0.3,
        };
    } else if rumble_strength < 2.0 {
        rumble = GamepadRumbleIntensity {
            strong_motor: 0.2,
            weak_motor: 0.1,
        };
    } else {
        rumble = GamepadRumbleIntensity {
            strong_motor: 0.1,
            weak_motor: 0.1,
        };
    }

    rumble_strength = (2.0 - rumble_strength.min(2.0)) / 10.0;
    if rumble_strength > 0.0 {
        for mut shake in &mut shakes {
            for gamepad in gamepads.iter() {
                shake.add_trauma(rumble_strength);
                *wait_time = time.elapsed_seconds();
                rumble_requests.send(GamepadRumbleRequest::Add {
                    duration: Duration::from_secs_f32(rumble_strength),
                    intensity: rumble,
                    gamepad,
                });
            }
        }
    }
}

pub fn resolve_damage(
    mut damage_events: EventReader<DamageEvent>,
    mut health_query: Query<&mut Health>,
) {
    for de in damage_events.read() {
        if let Ok(mut hp) = health_query.get_mut(de.0) {
            if hp.1 >= de.1 {
                hp.1 -= de.1;
            } else {
                hp.1 = 0;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn camera_follow(
    mut gizmos: Gizmos,
    player_settings: Res<PlayerSettings>,
    player_query: Query<(&Transform, &Thrust), With<Player>>,
    mut camera_query: Query<
        (&Camera, &mut Transform, &mut GameCameraTarget),
        (Without<Player>, With<GameCamera>),
    >,
    mut panning: Local<bool>,
    mut panning_time: Local<f32>,
) {
    let Ok((player_transform, Thrust(thrust))) = player_query.get_single() else {
        return;
    };

    let Ok((_camera, mut camera_transform, mut target)) = camera_query.get_single_mut() else {
        return;
    };

    let distance = camera_transform
        .translation
        .distance(player_transform.translation);

    if !*panning {
        *panning = distance > player_settings.camera_deadzone;
        if *panning {
            *panning_time = 0.0;
        } else {
            *panning_time = (*panning_time - 0.1).clamp(0.0, 1.0);
        }
    } else {
        *panning_time = (*panning_time + 0.05).clamp(0.0, 1.0);
    }

    let extra_radius = smooth_function(*panning_time * *thrust, 3.0) * player_settings.scan_radius;

    if player_settings.show_gizmos {
        gizmos
            .circle_2d(
                camera_transform.translation.xy(),
                player_settings.camera_deadzone + extra_radius,
                Color::WHITE
                    .with_a((1.0 - smooth_function(*panning_time * *thrust, 3.0)).clamp(0.0, 1.0)),
            )
            .segments(128);
    }

    if *panning {
        let movement_direction = player_transform.rotation * Vec3::Y;
        let radius = player_settings.scan_radius;
        let movement =
            movement_direction.xy() * (radius + player_settings.camera_deadzone) * 0.5 * *thrust;
        let movement_str = movement
            .length()
            .min((player_settings.camera_offset + player_settings.camera_deadzone) * thrust);
        let movement_vec = Vec2::ZERO;
        let mut new_movement = movement_vec * movement_str;
        if new_movement.is_nan() {
            new_movement = Vec2::ZERO;
        }

        let center = player_transform.translation.xy() + new_movement;

        target.0 = Vec3::new(center.x, center.y, camera_transform.translation.z);
        camera_transform.translation = camera_transform.translation.lerp(
            target.0,
            player_settings.camera_speed * smooth_function(*panning_time, 0.1),
        );

        if *thrust < 0.7 {
            *panning = false;
        }
    }
}

fn show_debug_window(
    mut player_settings: ResMut<PlayerSettings>,
    mut context: NonSendMut<ImguiContext>,
    mut next_state: ResMut<NextState<GameStates>>,
    mut toggle_ui: EventWriter<ToggleUI<HpBar>>,
) {
    let ui = context.ui();
    if player_settings.show_debug {
        let mut opened = true;
        ui.window("Player Settings")
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .save_settings(true)
            .opened(&mut opened)
            .build(|| {
                ui.group(|| {
                    ui.input_float("Deadzone", &mut player_settings.camera_deadzone)
                        .build();
                    ui.input_float("Offset", &mut player_settings.camera_offset)
                        .build();
                    ui.input_float("Speed", &mut player_settings.camera_speed)
                        .build();
                });
                ui.separator();
                ui.group(|| {
                    ui.input_float("Missile Cooldown", &mut player_settings.missile_cooldown)
                        .build();
                    ui.input_float("Missile Lifetime", &mut player_settings.missile_lifetime)
                        .build();
                    ui.input_float("Missile Angle", &mut player_settings.missile_angle)
                        .build();
                    ui.input_int("Missile Count", &mut player_settings.missile_count)
                        .build();
                    ui.input_float("Railgun Cooldown", &mut player_settings.railgun_cooldown)
                        .build();
                    ui.input_float("Railgun Range", &mut player_settings.railgun_range)
                        .build();
                });
                ui.separator();
                ui.input_float("Scan radius", &mut player_settings.scan_radius)
                    .build();
                ui.separator();
                if ui.button("[F2] Toggle HP bar") {
                    toggle_ui.send(ToggleUI::<HpBar>::default());
                }

                ui.checkbox("[F3] Use rumble?", &mut player_settings.use_rumble);
                ui.input_float("Rumble interval", &mut player_settings.time_between_rumbles)
                    .build();
                ui.checkbox("[F4] Show gizmos?", &mut player_settings.show_gizmos);
                if ui.button("[F5] Reload level") {
                    next_state.set(GameStates::AssetLoading);
                }
            });

        if !opened {
            player_settings.show_debug = false;
        }
    }
}
