use avian2d::prelude::*;
use bevy::{ecs::system::EntityCommands, prelude::*};
use lightyear::{avian2d::prelude::LightyearAvianPlugin, prelude::*};

use project_protocol::{BodyMotion, CircleBody};

// Install after ProtocolPlugin. Avian owns simulation poses.
pub(super) fn install_physics(app: &mut App) {
    app.insert_resource(Gravity::ZERO).add_plugins((
        PhysicsPlugins::default()
            .with_length_unit(100.0)
            .build()
            .disable::<PhysicsTransformPlugin>()
            .disable::<PhysicsInterpolationPlugin>(),
        LightyearAvianPlugin {
            // Avian's persistent island and broad-phase indices are internally linked.
            // Restoring their histories independently can leave dangling StableVec keys.
            // Keep the live derived caches and replay corrected body state through them.
            rollback_resources: false,
            ..default()
        },
    ));

    // The integration's velocity-aware Hermite bundle has priority 4. At abrupt contact
    // stops its tangents can overshoot into the other body. Linear pose sampling stays
    // between the solved endpoints instead. Both bodies use the same frame timeline.
    app.linear_interpolate_with_priority::<Position>(10);
    app.linear_interpolate_with_priority::<Rotation>(10);
}

pub(super) fn prepare_authoritative_body(
    trigger: On<Insert, CircleBody>,
    bodies: Query<&CircleBody, Without<Collider>>,
    mut commands: Commands,
) {
    if let Ok(circle) = bodies.get(trigger.entity) {
        insert_body(&mut commands.entity(trigger.entity), circle);
    }
}

pub(super) fn prepare_predicted_body(
    trigger: On<Insert, (CircleBody, Predicted)>,
    bodies: Query<&CircleBody, (With<Predicted>, Without<Collider>)>,
    mut commands: Commands,
) {
    if let Ok(circle) = bodies.get(trigger.entity) {
        insert_body(&mut commands.entity(trigger.entity), circle);
        commands.entity(trigger.entity).insert(FrameInterpolate);
    }
}

fn insert_body(entity: &mut EntityCommands, circle: &CircleBody) {
    assert!(
        circle.radius.is_finite() && circle.radius > 0.0,
        "circle radius must be finite and positive"
    );

    entity.insert((
        match circle.motion {
            BodyMotion::Dynamic => RigidBody::Dynamic,
            BodyMotion::Static => RigidBody::Static,
        },
        Collider::circle(circle.radius),
        // A half-pixel skin absorbs the soft solver's small contact penetration while
        // keeping the gameplay/debug circles separated under sustained movement input.
        CollisionMargin(0.5),
        // Facing is input-driven; contact impulses must not spin the ship.
        LockedAxes::ROTATION_LOCKED,
        Friction::ZERO,
        Restitution::ZERO,
    ));
}
