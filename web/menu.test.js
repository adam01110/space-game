const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");

const source = fs
  .readFileSync("web/menu.js", "utf8")
  .replace(/^import init from .*;\n/, "");
const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

async function testMenu({
  clickBeforeDownload = false,
  downloadFails = false,
  initFails = false,
} = {}) {
  let click;
  let notifyReady;
  let resolveDownload;
  let rejectDownload;
  let initCalls = 0;
  const classes = new Set();
  const button = {
    disabled: false,
    addEventListener(_event, handler) {
      click = handler;
    },
  };
  const status = {
    textContent: "Downloading game…",
    classList: {
      add(name) {
        classes.add(name);
      },
    },
  };
  const shell = {
    playerReady: false,
    getAttribute() {
      return this.playerReady ? "true" : "false";
    },
  };
  const elements = {
    "#game-shell": shell,
    "#play": button,
    "#status": status,
  };
  vm.runInNewContext(source, {
    document: {
      querySelector: (selector) => elements[selector],
      body: {
        classList: {
          add(name) {
            classes.add(name);
          },
          contains(name) {
            return classes.has(name);
          },
        },
      },
    },
    MutationObserver: class {
      constructor(callback) {
        notifyReady = callback;
      }
      observe() {}
    },
    fetch: () =>
      new Promise((resolve, reject) => {
        resolveDownload = resolve;
        rejectDownload = reject;
      }),
    init: async () => {
      initCalls++;
      if (initFails) throw new Error("startup failed");
    },
    console: { error() {} },
  });

  assert.equal(initCalls, 0, "page load must not start Bevy");
  assert.equal(classes.has("entered"), false);
  if (clickBeforeDownload) void click();
  if (downloadFails) {
    rejectDownload(new Error("offline"));
    await tick();
    assert.equal(button.disabled, true);
    assert.match(status.textContent, /Failed to download/);
    assert.equal(initCalls, 0);
    return;
  }
  resolveDownload({ ok: true, arrayBuffer: async () => new ArrayBuffer(4) });
  await tick();
  if (!clickBeforeDownload) {
    assert.equal(status.textContent, "");
    void click();
  }
  void click();
  await tick();
  assert.equal(initCalls, 1, "Play must start Bevy exactly once");
  if (initFails) {
    assert.match(status.textContent, /Failed to start/);
    assert.equal(classes.has("entered"), false);
    return;
  }
  assert.equal(
    classes.has("entered"),
    false,
    "menu must wait for a local player",
  );
  shell.playerReady = true;
  notifyReady();
  assert.equal(classes.has("entered"), true, "menu must reveal a ready client");
}

(async () => {
  await testMenu();
  await testMenu({ clickBeforeDownload: true });
  await testMenu({ downloadFails: true });
  await testMenu({ initFails: true });
  console.log(
    "menu states passed: download, Play, readiness, duplicate click, failures",
  );
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
