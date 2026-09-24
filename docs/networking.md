# Networking under latency and suspension

## Implemented policy

- The simulation stays at 60 Hz. All solid players and the arena remain
  predicted and replicated to every client; hiding a collider would break
  same-tick contact prediction.
- Client prediction and rollback limits are both 90 ticks (1.5 seconds),
  replacing a 20-tick rollback limit below the observed 27–28 tick
  corrections. This is a bounded budget, not a guarantee for arbitrary
  latency; longer replays cost CPU and retain more history.
- Timeline synchronization uses a three-tick fixed jitter margin (50 ms at
  60 Hz) plus its measured-jitter allowance. Local input stays undelayed.
- Input packets carry 15 ticks of redundant history instead of five, about
  250 ms at the current send cadence. This raises upstream and rebroadcast
  traffic; it does not recover arbitrary outages or remove prediction
  corrections.
- A frame gap of at least 250 ms retires stale client state before `PreUpdate`
  processes old packets or replays old inputs. The threshold matches Bevy's
  default virtual-time clamp. Hidden browser tabs wait for visibility before
  requesting new credentials; throttled unfocused native windows wait for
  focus; normally updating unfocused windows stay connected.
- Retirement clears received/predicted entities, unmatched prespawns, disabled
  shots, input events, held buttons, counters, and synchronization/prediction
  bookkeeping. A fresh guest connection resets the player instead of preserving
  a session. A lost disconnect packet can leave the old server player until its
  timeout.
- Disconnect reasons are logged. Rollback rejection is not a transport
  disconnect. The Netcode 15-second connection timeout and WebTransport's idle
  timeout are unchanged.

## Projectiles

- Only the controlling client prespawns a shot. The server replicates it with
  prediction enabled for every interested client, keeping projectiles on the
  same timeline as predicted players. Other clients do not invent duplicate
  prespawns from remote input.
- Shot matching uses player identity plus the consumed click counter, not the
  simulation tick on which a click arrived. The one-second shot lifetime and
  five-second reload bound reuse of the wrapping action counter.
- Predicted expiry uses `prediction_despawn`, preserving history for rollback.
  Only predicted/prespawned shots advance on the client; confirmed-only state is
  untouched. Lightyear expires unmatched prespawns after 50 ticks (about
  833 ms), and the larger rollback budget does not extend that window, so late
  spawn delivery can replace an expired local shot.
- Visibility uses an entry radius of 4096 world units and an exit radius of
  4608, measured from the recipient's authoritative player position. The owner
  always receives its shots, and hidden remote shots are removed rather than
  retained. These are fixed world-space radii, not negotiated viewport
  dimensions, so extreme zoom-out or large viewports can see the interest
  boundary. The scan is O(players × projectiles), not a spatial index.
- `ReplicatePriority(0.5)` lowers projectile mutation eligibility under fast
  acknowledgments. It does not deprioritize reliable spawns/despawns, impose a
  bandwidth quota, or guarantee half the traffic under latency. Player and arena
  priority are unchanged.

## Bandwidth and delta compression

Lightyear 0.29 / Replicon 0.42 record component diffs and send those since each
recipient's acknowledged cursor, but small, frequently changing components are
not automatically cheaper. A Postcard experiment measured:

| Payload (excluding replication headers) | Bytes |
| --- | ---: |
| Full position + direction + lifetime | 17 |
| Position + lifetime | 9 |
| One recorded position/lifetime patch in a sequence | 10 |
| Twelve outstanding patches | 109 |
| Twenty-four outstanding patches | 217 |

Immutable direction is therefore a separate `BlasterTrajectory` component, which
leaves moving `BlasterShot` state at nine payload bytes. This is a component
payload reduction, not a measured whole-connection reduction. Idle
blaster/reload/counter/beam state uses value-aware change detection, so visiting
it in simulation does not mark it for retransmission.

Per-tick `replicate_diff` is intentionally not enabled; large future collections
(trails, inventory) are the better candidates. Physics floats are not quantized.
The component schema changed, so client and server builds must match.

## Combat boundary

There is no weapon hit/damage system. The FPS example rewinds interpolated
colliders; this game predicts all player colliders, so an interpolation-delay
rewind would mix timelines. Authoritative swept projectile hits on the existing
prediction timeline can be added without switching player prediction, but
rewind-based combat requires an explicit target-timeline design.

## Verification

`crates/client/src/tests/network/wire.rs` drives real Lightyear serialization,
transport packets, inputs, synchronization, replication, and prediction between
two headless Bevy apps. A deterministic wire delays server-to-client packets by
400 ms and optionally drops every tenth packet in each direction, including
acknowledgments. It checks convergence after movement stops, an abrupt latency
increase, prespawn entity continuity, cleanup, initial projectile invisibility,
reveal, hide, and owner visibility, waiting for delivery within a bounded window
instead of assuming reliable retransmission timing. It is not a QUIC/WebTransport
handshake or browser-rendering test.

Additional tests cover coordinated rollback limits, suspension policy, cleanup
before packet receive, input redundancy, prediction-safe expiry, untouched
confirmed copies, and idle component change detection. Native recovery fixtures
model a session started through Play; a separate regression test ensures a slow
frame before Play cannot start a guest connection.

```sh
nix develop --command cargo test --workspace --lib --bins --features space-game-client/dev,space-game-server/dev
nix develop --command cargo check -p space-game-client --target wasm32-unknown-unknown
nix develop --command cargo clippy --workspace --all-targets -- -D warnings
nix fmt
```

For real transport testing:

```sh
just netem 400ms 10%
just netem-reset
```

`netem` impairs loopback IPv4 UDP on port 5000 in both directions, including
QUIC traffic. The `latency` argument is a round-trip figure matching an in-game
ping display, so the recipe splits it into a 200 ms one-way delay per direction;
`loss` applies independently per direction. HTTP credential traffic is TCP 5001
and is not delayed. The recipe replaces the loopback root qdisc and was not run
as part of automated verification. Manual validation still needed: live browser
tab suspension, compositor throttling, CPU cost of long rollbacks,
randomized/burst loss, and actual transport disconnect causes. Transient
late-input diagnostics can still occur when latency jumps.
