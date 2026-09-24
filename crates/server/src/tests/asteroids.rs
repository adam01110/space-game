use std::time::Duration;

use avian2d::prelude::Position;
use bevy::{prelude::*, state::app::StatesPlugin};
use lightyear::prelude::server::ServerPlugins;

use space_game_protocol::{
    ArenaBoundary, Asteroid, AsteroidHealth, CircleBody, Player, ProtocolPlugin,
};

use crate::asteroids::ServerAsteroidPlugin;

const INNER_COUNT: usize = 12;
const OUTER_COUNT: usize = 180;

fn assert_no_overlaps(rocks: &[(Vec2, f32)]) {
    for (index, (position, radius)) in rocks.iter().enumerate() {
        for (other, other_radius) in rocks.iter().skip(index + 1) {
            assert!(position.distance(*other) > radius + other_radius);
        }
    }
}

#[test]
fn server_populates_both_sides_of_border_without_overlaps_and_replenishes() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        ServerPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / 60.0),
        },
        ProtocolPlugin,
        ServerAsteroidPlugin,
    ));
    app.world_mut().spawn(ArenaBoundary::new(1300.0));
    app.world_mut()
        .spawn((Player, Position(Vec2::ZERO), CircleBody::dynamic(23.4)));
    app.world_mut().flush();

    let mut rocks = app
        .world_mut()
        .query::<(Entity, &Position, &Asteroid, &AsteroidHealth)>();
    let found: Vec<_> = rocks
        .iter(app.world())
        .map(|(entity, pos, rock, health)| {
            assert_eq!(health.0, AsteroidHealth::FULL);
            (entity, pos.0, rock.radius)
        })
        .collect();
    assert_eq!(found.len(), INNER_COUNT + OUTER_COUNT);
    let outside = found
        .iter()
        .filter(|(_, pos, radius)| pos.length() > 1300.0 + radius)
        .count();
    assert_eq!(outside, OUTER_COUNT);
    for (index, (_, pos, radius)) in found.iter().enumerate() {
        for (_, other, other_radius) in found.iter().skip(index + 1) {
            assert!(pos.distance(*other) > radius + other_radius);
        }
    }

    // Growing the arena must not remove rocks it overtakes, and must build a new
    // exterior warning band. Damage-related despawns are replenished separately.
    app.world_mut()
        .query::<&mut ArenaBoundary>()
        .single_mut(app.world_mut())
        .unwrap()
        .radius = 1800.0;
    app.world_mut()
        .resource_mut::<Time<Fixed>>()
        .advance_by(Duration::from_secs(3));
    app.world_mut().run_schedule(FixedPostUpdate);
    assert!(found
        .iter()
        .all(|(entity, _, _)| app.world().get_entity(*entity).is_ok()));
    // The original exterior rocks move with the border, preserving their identity,
    // direction and distance from the pylons instead of being replaced.
    for (entity, old_position, _) in found
        .iter()
        .filter(|(_, position, radius)| position.length() > 1300.0 + radius)
    {
        let current = app.world().get::<Position>(*entity).unwrap().0;
        assert!((current.length() - old_position.length() - 500.0).abs() < 0.01);
        assert!(current.distance(old_position.normalize() * current.length()) < 0.01);
    }
    let new_outside = rocks
        .iter(app.world())
        .filter(|(_, pos, rock, _)| pos.0.length() > 1800.0 + rock.radius)
        .count();
    assert!(new_outside >= 249);
    let new_inside = rocks
        .iter(app.world())
        .filter(|(_, pos, rock, _)| pos.0.length() + rock.radius < 1800.0)
        .count();
    assert!(new_inside >= 23);
    let after_growth: Vec<_> = rocks
        .iter(app.world())
        .map(|(_, pos, rock, _)| (pos.0, rock.radius))
        .collect();
    assert_no_overlaps(&after_growth);

    let count_before_damage = rocks.iter(app.world()).count();
    app.world_mut().entity_mut(found[0].0).despawn();
    app.world_mut()
        .resource_mut::<Time<Fixed>>()
        .advance_by(Duration::from_secs(3));
    app.world_mut().run_schedule(FixedPostUpdate);
    assert_eq!(rocks.iter(app.world()).count(), count_before_damage);

    app.world_mut()
        .query::<&mut ArenaBoundary>()
        .single_mut(app.world_mut())
        .unwrap()
        .radius = 1300.0;
    app.world_mut()
        .resource_mut::<Time<Fixed>>()
        .advance_by(Duration::from_secs(3));
    app.world_mut().run_schedule(FixedPostUpdate);
    let near_border = rocks
        .iter(app.world())
        .filter(|(_, pos, rock, _)| {
            let distance = pos.0.length();
            distance >= 1300.0 + rock.radius + 24.0
                && distance <= 1300.0 + rock.radius + 32.0 + 850.0
        })
        .count();
    assert!(near_border >= OUTER_COUNT);
}
