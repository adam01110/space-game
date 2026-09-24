use std::collections::{HashMap, HashSet};

use bevy::{
    camera::CameraUpdateSystems, ecs::system::SystemParam, prelude::*, transform::TransformSystems,
};
use bevy_extended_ui::styles::CssID;
use lightyear::prelude::input::native::InputMarker;

use space_game_game::PLAYER_RADIUS;
use space_game_protocol::{
    Asteroid, AsteroidHealth, CircleBody, Player, PlayerHealth, PlayerInput,
};

use crate::{
    camera::{CanvasCamera, GameplayCamera, GameplayCanvas},
    palette::Palette,
};

const BAR_HEIGHT: f32 = 4.0;
const BAR_GAP: f32 = 4.0;

type RemoteShips<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        Option<&'static PlayerHealth>,
        Option<&'static AsteroidHealth>,
        Option<&'static CircleBody>,
    ),
    (
        Or<(With<Player>, With<Asteroid>)>,
        Without<InputMarker<PlayerInput>>,
    ),
>;

#[derive(Component)]
pub(super) struct RemoteHealthBar;

pub(super) struct RemoteHealthPlugin;

impl Plugin for RemoteHealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            update_remote_health_bars
                .after(CameraUpdateSystems)
                .after(TransformSystems::Propagate),
        );
    }
}

// Gameplay renders to an image, then the canvas camera displays that image in the window.
#[derive(SystemParam)]
struct BarView<'w, 's> {
    window: Single<'w, 's, &'static Window, With<bevy::window::PrimaryWindow>>,
    gameplay: Single<'w, 's, (&'static Camera, &'static GlobalTransform), With<GameplayCamera>>,
    canvas: Single<'w, 's, (&'static Camera, &'static GlobalTransform), With<CanvasCamera>>,
    image: Single<'w, 's, &'static GlobalTransform, With<GameplayCanvas>>,
}

impl BarView<'_, '_> {
    fn project(&self, world: Vec3) -> Option<Vec2> {
        let on_image = self
            .gameplay
            .0
            .world_to_viewport(self.gameplay.1, world)
            .ok()?;

        let size = self.gameplay.0.logical_target_size()?;
        let on_sprite = Vec3::new(on_image.x - size.x / 2.0, size.y / 2.0 - on_image.y, 0.0);
        let on_canvas = self.image.transform_point(on_sprite);

        self.canvas
            .0
            .world_to_viewport(self.canvas.1, on_canvas)
            .ok()
    }

    fn layout(&self, ship: Vec3, current: u8, full: u8, radius: f32) -> Option<BarLayout> {
        let top = ship.y - radius - BAR_GAP;
        let width = radius * 2.0 * 0.75;
        let first = self.project(Vec3::new(ship.x - width / 2.0, top, ship.z))?;
        let second = self.project(Vec3::new(ship.x + width / 2.0, top - BAR_HEIGHT, ship.z))?;

        Some(BarLayout::between(first, second, current, full))
            .filter(|layout| layout.overlaps(self.window.size()))
    }
}

struct BarLayout {
    position: Vec2,
    size: Vec2,
    fill_width: f32,
}

impl BarLayout {
    fn between(first: Vec2, second: Vec2, current: u8, full: u8) -> Self {
        let size = (second - first).abs();

        Self {
            position: first.min(second),
            size,
            fill_width: size.x * f32::from(current.min(full)) / f32::from(full),
        }
    }

    fn overlaps(&self, window: Vec2) -> bool {
        let end = self.position + self.size;
        self.position.x <= window.x && self.position.y <= window.y && end.x >= 0.0 && end.y >= 0.0
    }

    const fn apply(&self, track: &mut Node, fill: &mut Node) {
        track.left = Val::Px(self.position.x);
        track.top = Val::Px(self.position.y);
        track.width = Val::Px(self.size.x);
        track.height = Val::Px(self.size.y);
        fill.width = Val::Px(self.fill_width);
        fill.height = Val::Px(self.size.y);
    }
}

#[derive(Clone, Copy)]
struct BarEntities {
    track: Entity,
    fill: Entity,
}

#[derive(Default)]
struct BarState(HashMap<Entity, BarEntities>);

impl BarState {
    fn show(
        &mut self,
        commands: &mut Commands,
        nodes: &mut Query<&mut Node>,
        root: Entity,
        player: Entity,
        layout: &BarLayout,
    ) {
        let existing = self
            .0
            .get(&player)
            .and_then(|bar| nodes.get_many_mut((bar.track, bar.fill).into()).ok());

        match existing {
            Some([mut track, mut fill]) => layout.apply(&mut track, &mut fill),
            _ => self.spawn(commands, root, player, layout),
        }
    }

    fn spawn(&mut self, commands: &mut Commands, root: Entity, player: Entity, layout: &BarLayout) {
        let mut track_node = Node {
            position_type: PositionType::Absolute,
            ..default()
        };
        let mut fill_node = Node::default();
        layout.apply(&mut track_node, &mut fill_node);

        let track = commands
            .spawn((
                RemoteHealthBar,
                track_node,
                BackgroundColor(Palette::Tan.color()),
                Pickable::IGNORE,
            ))
            .id();
        let fill = commands
            .spawn((
                fill_node,
                BackgroundColor(Palette::Sand.color()),
                Pickable::IGNORE,
            ))
            .id();

        commands.entity(track).add_child(fill);
        commands.entity(root).add_child(track);

        self.0.insert(player, BarEntities { track, fill });
    }

    fn remove_unseen(&mut self, commands: &mut Commands, seen: &HashSet<Entity>) {
        self.0.retain(|player, bar| match seen.contains(player) {
            true => true,
            false => {
                commands.entity(bar.track).despawn();
                false
            }
        });
    }
}

fn update_remote_health_bars(
    mut commands: Commands,
    view: BarView,
    local: Query<(), (With<Player>, With<InputMarker<PlayerInput>>)>,
    remotes: RemoteShips,
    roots: Query<(Entity, &CssID)>,
    mut nodes: Query<&mut Node>,
    mut bars: Local<BarState>,
) {
    let root = roots
        .iter()
        .find(|(_, id)| id.0 == "hud-remote-health")
        .map(|(entity, _)| entity)
        .filter(|_| !local.is_empty());
    let mut seen = HashSet::new();

    if let Some(root) = root {
        for (player, layout) in
            remotes
                .iter()
                .filter_map(|(player, transform, health, asteroid, body)| {
                    let (current, full) = if let Some(health) = health {
                        (health.0, PlayerHealth::FULL)
                    } else {
                        let current = asteroid?.0;
                        if current >= AsteroidHealth::FULL {
                            return None;
                        }
                        (current, AsteroidHealth::FULL)
                    };
                    view.layout(
                        transform.translation(),
                        current,
                        full,
                        body.map_or(PLAYER_RADIUS, |body| body.radius),
                    )
                    .map(|layout| (player, layout))
                })
        {
            seen.insert(player);
            bars.show(&mut commands, &mut nodes, root, player, &layout);
        }
    }

    bars.remove_unseen(&mut commands, &seen);
}
