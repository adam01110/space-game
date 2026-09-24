import init from "./space_game_client.js";

function element<T extends Element>(selector: string): T {
  const found = document.querySelector<T>(selector);
  if (found === null) throw new Error(`Missing element: ${selector}`);
  return found;
}

const shell = element<HTMLElement>("#game-shell");
const play = element<HTMLButtonElement>("#play");
const status = element<HTMLElement>("#status");

let requested = false;
let downloaded = false;
let failed = false;

function fail(error: unknown, message: string): void {
  console.error(message, error);
  failed = true;
  play.disabled = true;
  status.classList.add("failed");
  status.textContent = message;
}

// Fetch early, but leave client initialization until Play is clicked.
const wasmDownload: Promise<ArrayBuffer | null> = fetch("./space_game_client_bg.wasm")
  .then((response) => {
    if (!response.ok) throw new Error(`Wasm download: HTTP ${response.status}`);
    return response.arrayBuffer();
  })
  .then((bytes) => {
    downloaded = true;
    status.textContent = requested ? "Starting game…" : "";
    return bytes;
  })
  .catch((error: unknown) => {
    fail(error, "Failed to download game. Check the browser console.");
    return null;
  });

function enterWhenReady(): void {
  if (requested && !failed && shell.dataset.gameReady === "true") {
    document.body.classList.add("entered");
  }
}

new MutationObserver(enterWhenReady).observe(shell, {
  attributes: true,
  attributeFilter: ["data-game-ready"],
});

document.addEventListener("contextmenu", (event) => {
  // Right-click on the canvas fires the phase beam.
  if (event.target instanceof HTMLCanvasElement) event.preventDefault();
});

play.addEventListener("click", async () => {
  if (play.disabled) return;
  requested = true;
  play.disabled = true;
  enterWhenReady();
  if (downloaded) status.textContent = "Starting game…";

  const bytes = await wasmDownload;
  if (bytes === null) return;

  try {
    await init({ module_or_path: bytes });
    status.textContent = "Waiting for your player…";
  } catch (error) {
    fail(error, "Failed to start game. Check the browser console.");
  }
});
