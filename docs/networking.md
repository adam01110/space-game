# Networking under latency and suspension

## Implemented policy

The simulation remains 60 Hz. All solid players and the arena remain predicted and
replicated to every client; hiding a collider would break same-tick contact prediction.

- Client prediction and rollback limits are both **90 ticks (1.5 seconds)**. The old
  default rollback limit was 20 ticks, below the observed 27–28 tick corrections.
  This is a bounded budget, not a guarantee for arbitrary latency. Longer replays
  consume more CPU and retain more history.
- Timeline synchronization has a three-tick fixed jitter margin (50 ms at 60 Hz),
  plus its measured-jitter allowance. Local input remains undelayed.
- Input packets include 15 ticks of redundant history instead of five, approximately
  250 ms at the current send cadence. This increases upstream and rebroadcast traffic;
  it does not recover arbitrary outages or remove the need for prediction corrections.
- A real frame gap of at least 250 ms retires stale client state before `PreUpdate`
  can process old packets or replay old inputs. This threshold matches Bevy's default
  virtual-time clamp. Hidden browser tabs wait for visibility before requesting new
  credentials. Throttled, unfocused native windows wait for focus. Normally updating
  unfocused windows remain connected.
- Retirement clears received/predicted entities, unmatched prespawns, disabled shots,
  input events, held buttons, counters, and synchronization/prediction bookkeeping.
  A fresh guest connection resets the player rather than preserving a session.
  If a disconnect packet is lost, the old server player may remain until its timeout.
- Disconnect reasons are logged. Rollback rejection itself is not a transport
  disconnect. Netcode's 15-second connection timeout and WebTransport's separate
  idle timeout were not changed.

## Projectiles

Only the controlling client prespawns a shot. The server replicates it with prediction
enabled for every interested client, keeping projectiles on the same timeline as
predicted players. Other clients do not invent duplicate prespawns from remote input.

Shot matching uses player identity plus the consumed click counter, not the simulation
tick on which a click happened to arrive. The packet test reproduced different
client/server tick-based hashes and duplicate shots before this change. The one-second
shot lifetime and five-second reload bound reuse of the wrapping action counter.
Predicted expiry uses `prediction_despawn`, preserving history for rollback. Only
predicted/prespawned shots are advanced by the client; confirmed-only state is untouched.

Projectile visibility uses an entry radius of 4096 world units and an exit radius of
4608, measured from the recipient's authoritative player position. The owner always
receives its shots. Hidden remote shots are removed, not retained in a stale state.
These are conservative fixed world-space radii, not negotiated viewport dimensions;
extreme zoom-out or unusually large viewports can see the interest boundary. The
current scan is O(players × projectiles), not a spatial index.

`ReplicatePriority(0.5)` lowers projectile mutation eligibility under fast acknowledgments.
It does not deprioritize reliable spawns/despawns, impose a bandwidth quota, or guarantee
half the traffic under latency. Player and arena priority are unchanged.

## Bandwidth and delta compression

The installed Lightyear 0.29 / Replicon 0.42 APIs support recorded component diffs, but
that does not make small, frequently changing components cheaper automatically.
Replicon sends the recorded diffs since each recipient's acknowledged cursor.

A Postcard payload experiment with the original shot fields measured:

| Payload (excluding replication headers) | Bytes |
| --- | ---: |
| Full position + direction + lifetime | 17 |
| Position + lifetime | 9 |
| One recorded position/lifetime patch in a sequence | 10 |
| Twelve outstanding patches | 109 |
| Twenty-four outstanding patches | 217 |

Instead of adding that backlog, immutable direction is now a separate
`BlasterTrajectory` component. Moving `BlasterShot` state is nine payload bytes.
This is a component payload reduction, **not** a measured whole-connection reduction.
Idle blaster/reload/counter/beam state uses value-aware change detection so merely
visiting it in simulation does not mark it for retransmission.

Per-tick `replicate_diff` is intentionally not enabled. Large future collections
(trails, inventory, etc.) would be more appropriate candidates. Physics floats have
not been quantized. The component schema changed: deploy matching client/server builds.

## Combat boundary

There is still no weapon hit/damage system. The FPS example rewinds interpolated
colliders; this game predicts all player colliders. Applying an interpolation-delay
rewind to those players would mix timelines. Authoritative swept projectile hits on
the existing prediction timeline can be added without switching player prediction;
rewind-based combat requires an explicit target-timeline design. No damage values or
respawn rules were introduced here.

## Verification

`crates/client/src/network/tests/wire.rs` drives real Lightyear serialization,
transport packets, inputs, synchronization, replication, and prediction between two
headless Bevy apps. A deterministic wire delays server-to-client packets by 400 ms
and optionally drops every tenth packet in each direction, including acknowledgments.
It checks convergence after movement stops, an abrupt latency increase, prespawn entity
continuity, cleanup, initial projectile invisibility, reveal, hide, and owner visibility.
It is not a QUIC/WebTransport handshake or browser-rendering test.

Additional tests cover coordinated rollback limits, suspension policy, cleanup before
packet receive, input redundancy, prediction-safe expiry, untouched confirmed copies,
and idle component change detection.

```sh
nix develop --command cargo test --workspace --lib --bins --features space-game-client/dev,space-game-server/dev
nix develop --command cargo check -p space-game-client --target wasm32-unknown-unknown
nix develop --command cargo clippy --workspace --all-targets -- -D warnings
nix fmt
```

For real transport testing, `just netem 400ms 10%` currently impairs only loopback
IPv4 UDP **source port 5000**: server-to-client traffic, including QUIC traffic.
It adds roughly 400 ms RTT, not 800 ms, and does not delay HTTP credentials or the
client-to-server direction. `just netem-reset` removes it. The recipe replaces the
loopback root qdisc; it was left unchanged and was not run as part of the automated
verification. Live browser tab suspension, compositor throttling, CPU cost of long
rollbacks, randomized/burst loss, and actual transport disconnect causes remain manual
validation tasks. Transient late-input diagnostics can still occur when latency jumps.

## Implementation files

Paths are relative to `/home/adam0/Projects/space-game`:

- `/home/adam0/Projects/space-game/crates/client/src/network.rs`
- `/home/adam0/Projects/space-game/crates/client/src/network/policy.rs`
- `/home/adam0/Projects/space-game/crates/client/src/network/recovery.rs`
- `/home/adam0/Projects/space-game/crates/client/src/network/connection.rs`
- `/home/adam0/Projects/space-game/crates/client/src/network/tests.rs`
- `/home/adam0/Projects/space-game/crates/client/src/network/tests/wire.rs`
- `/home/adam0/Projects/space-game/crates/client/src/input.rs`
- `/home/adam0/Projects/space-game/crates/client/src/blasters.rs`
- `/home/adam0/Projects/space-game/crates/client/Cargo.toml`
- `/home/adam0/Projects/space-game/crates/game/src/abilities.rs`
- `/home/adam0/Projects/space-game/crates/game/src/blasters.rs`
- `/home/adam0/Projects/space-game/crates/game/src/blasters/shots.rs`
- `/home/adam0/Projects/space-game/crates/game/src/phase_beam.rs`
- `/home/adam0/Projects/space-game/crates/game/src/tests.rs`
- `/home/adam0/Projects/space-game/crates/game/src/tests/blasters.rs`
- `/home/adam0/Projects/space-game/crates/protocol/src/components.rs`
- `/home/adam0/Projects/space-game/crates/protocol/src/lib.rs`
- `/home/adam0/Projects/space-game/crates/protocol/src/plugin.rs`
- `/home/adam0/Projects/space-game/crates/server/src/abilities.rs`
- `/home/adam0/Projects/space-game/crates/server/src/lib.rs`
- `/home/adam0/Projects/space-game/Cargo.lock`
- `/home/adam0/Projects/space-game/docs/networking.md`
