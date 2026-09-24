import init, { play } from "./space_game_client.js";

function element<T extends Element>(selector: string): T {
  const found = document.querySelector<T>(selector);
  if (found === null) throw new Error(`Missing element: ${selector}`);
  return found;
}

const shell = element<HTMLElement>("#game-shell");
const playButton = element<HTMLButtonElement>("#play");
const status = element<HTMLElement>("#status");

// The client owns the canvas while a session runs, and asks for the menu back through the shell
// element once the session ended.
let playing = false;
let booted = false;
let downloaded = false;
let failed = false;
let menuRequests = 0;
// Play clicks and the readiness the menu revealed the game for, so a client that stopped
// reporting readiness after a session cannot pass off its last state as a new one.
let plays = 0;
let revealed = 0;

function fail(error: unknown, message: string): void {
  console.error(message, error);
  failed = true;
  playButton.disabled = true;
  status.classList.add("failed");
  status.textContent = message;
}

function showMenu(): void {
  playing = false;
  playButton.disabled = false;
  status.classList.remove("failed");
  status.textContent = "";
  document.body.classList.remove("entered");
}

function enterWhenReady(): void {
  const ready = shell.dataset.gameReady === "true";
  if (playing && !failed && revealed < plays && ready) {
    revealed = plays;
    document.body.classList.add("entered");
  }
}

function observeShell(): void {
  const requests = Number(shell.dataset.menuRequests ?? 0);
  if (requests !== menuRequests) {
    menuRequests = requests;
    showMenu();
  }
  enterWhenReady();
}

// Fetch early, but leave client initialization until Play is clicked.
const wasmDownload: Promise<ArrayBuffer | null> = fetch("./space_game_client_bg.wasm")
  .then((response) => {
    if (!response.ok) throw new Error(`Wasm download: HTTP ${response.status}`);
    return response.arrayBuffer();
  })
  .then((bytes) => {
    downloaded = true;
    status.textContent = playing ? "Starting game…" : "";
    return bytes;
  })
  .catch((error: unknown) => {
    fail(error, "Failed to download game. Check the browser console.");
    return null;
  });

new MutationObserver(observeShell).observe(shell, {
  attributes: true,
  attributeFilter: ["data-game-ready", "data-menu-requests"],
});

document.addEventListener("contextmenu", (event) => {
  // Right-click on the canvas fires the phase beam.
  if (event.target instanceof HTMLCanvasElement) event.preventDefault();
});

playButton.addEventListener("click", async () => {
  if (playButton.disabled) return;
  playButton.disabled = true;
  plays += 1;
  playing = true;
  if (downloaded) status.textContent = "Starting game…";

  const bytes = await wasmDownload;
  if (bytes === null) return;

  try {
    if (!booted) {
      await init({ module_or_path: bytes });
      booted = true;
    }
    // The module starts its first session while it boots; it keeps running behind the menu, so
    // every later Play click has to ask it for another session.
    play();
    status.textContent = "Waiting for your player…";
  } catch (error) {
    fail(error, "Failed to start game. Check the browser console.");
  }
});
