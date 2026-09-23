# Main menu implementation

## Scope

One Play action and truthful loading status. The menu uses the Nebulaspace palette and has no settings, HUD, or other controls in this slice.

## Startup lifecycle

- Both browser builds package `web/index.html`, `web/menu.css`, and `web/menu.js`. HTML/CSS display the menu immediately. The Wasm binary begins downloading on page load, but Bevy initialization and the guest connection begin only after Play. Until the local player exists, the menu stays on screen with a loading status. A download or startup error remains visible instead of revealing the game.
- Native starts with a matching menu rendered by `bevy_extended_ui`. Native guest authentication/connection waits for Play and displays connection progress. Game-specific UI will use Bevy Extended UI; this slice does not add HUD controls.
- Native and browser gameplay behavior after starting remains unchanged. The browser page observes the Wasm-only `data-game-ready` signal from the local input-marked player.

## Verification

Check packaging for `just wasm` and `just web-build`, native and Wasm compilation, and browser states before Play, while downloading, after Play but before local player readiness, and on failure. The browser menu uses a solid background and does not create a WebGL context.
