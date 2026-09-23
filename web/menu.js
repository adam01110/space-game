import init from "./space_game_client.js";

const shell = document.querySelector("#game-shell");
const play = document.querySelector("#play");
const status = document.querySelector("#status");
let requested = false;
let downloaded = false;
let initialized = false;
let failed = false;

function ready() {
  return shell.getAttribute("data-game-ready") === "true";
}

function update() {
  if (failed) return;
  if (requested && ready()) {
    document.body.classList.add("entered");
    return;
  }
  status.textContent = !downloaded
    ? "Downloading game…"
    : !requested
      ? ""
      : !initialized
        ? "Starting game…"
        : "Waiting for your player…";
}

function fail(error, message) {
  console.error(message, error);
  failed = true;
  play.disabled = true;
  status.classList.add("failed");
  status.textContent = message;
}

// Download the Wasm immediately, without initializing Bevy or connecting yet.
const wasmDownload = fetch("./space_game_client_bg.wasm")
  .then((response) => {
    if (!response.ok) throw new Error(`Wasm download: HTTP ${response.status}`);
    return response.arrayBuffer();
  })
  .then((bytes) => {
    downloaded = true;
    update();
    return bytes;
  })
  .catch((error) => {
    fail(error, "Failed to download game. Check the browser console.");
    return null;
  });

new MutationObserver(update).observe(shell, {
  attributes: true,
  attributeFilter: ["data-game-ready"],
});
play.addEventListener("click", async () => {
  if (requested || failed) return;
  requested = true;
  play.disabled = true;
  update();
  const bytes = await wasmDownload;
  if (!bytes) return;
  try {
    // The wasm-bindgen start hook runs the client; Play is the only startup path.
    await init({ module_or_path: bytes });
    initialized = true;
    update();
  } catch (error) {
    fail(error, "Failed to start game. Check the browser console.");
  }
});
