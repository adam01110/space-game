import init from "./space_game_client.js";

function element<T extends Element>(selector: string): T {
  const found = document.querySelector<T>(selector);
  if (found === null) throw new Error(`Missing element: ${selector}`);
  return found;
}

const shell = element<HTMLElement>("#game-shell");
const play = element<HTMLButtonElement>("#play");
const status = element<HTMLElement>("#status");

// Right-click on the canvas goes to the phase beam, not the browser menu.
document.addEventListener("contextmenu", (event) => {
  if (event.target instanceof HTMLCanvasElement) event.preventDefault();
});
let requested = false;
let downloaded = false;
let initialized = false;
let failed = false;

function ready(): boolean {
  return shell.getAttribute("data-game-ready") === "true";
}

function update(): void {
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

function fail(error: unknown, message: string): void {
  console.error(message, error);
  failed = true;
  play.disabled = true;
  status.classList.add("failed");
  status.textContent = message;
}

// Download the Wasm immediately, without initializing Bevy or connecting yet.
const wasmDownload: Promise<ArrayBuffer | null> = fetch("./space_game_client_bg.wasm")
  .then((response) => {
    if (!response.ok) throw new Error(`Wasm download: HTTP ${response.status}`);
    return response.arrayBuffer();
  })
  .then((bytes) => {
    downloaded = true;
    update();
    return bytes;
  })
  .catch((error: unknown) => {
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
  if (bytes === null) return;
  try {
    // The wasm-bindgen start hook runs the client; Play is the only startup path.
    await init({ module_or_path: bytes });
    initialized = true;
    update();
  } catch (error) {
    fail(error, "Failed to start game. Check the browser console.");
  }
});
