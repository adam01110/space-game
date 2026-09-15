use bevy::prelude::*;
use lightyear::prelude::{NetworkTarget, Replicate};
use project_protocol::BlasterShot;

pub(super) struct ServerAbilitiesPlugin;

impl Plugin for ServerAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(replicate_blaster_shot);
    }
}

// Server-only replication policy stays outside the shared simulation crate.
fn replicate_blaster_shot(trigger: On<Add, BlasterShot>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert(Replicate::to_clients(NetworkTarget::All));
}
