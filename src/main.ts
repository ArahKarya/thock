import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Config {
  enabled: boolean;
  volume: number; // 0.0..=1.0
  pitch_variation: number; // 0.0..=1.0
  play_on_release: boolean;
  mouse_enabled: boolean;
  pack: string;
}

interface PackInfo {
  id: string;
  name: string;
  description: string;
}

const el = {
  enabled: document.querySelector<HTMLInputElement>("#enabled")!,
  mouse: document.querySelector<HTMLInputElement>("#mouse")!,
  pack: document.querySelector<HTMLSelectElement>("#pack")!,
  packDesc: document.querySelector<HTMLParagraphElement>("#pack-desc")!,
  volume: document.querySelector<HTMLInputElement>("#volume")!,
  volumeOut: document.querySelector<HTMLOutputElement>("#volume-out")!,
  pitch: document.querySelector<HTMLInputElement>("#pitch")!,
  pitchOut: document.querySelector<HTMLOutputElement>("#pitch-out")!,
  release: document.querySelector<HTMLInputElement>("#release")!,
  test: document.querySelector<HTMLButtonElement>("#test")!,
  hint: document.querySelector<HTMLParagraphElement>("#hint")!,
};

let packs: PackInfo[] = [];

const pct = (v: number): string => `${Math.round(v * 100)}%`;

function renderPackDesc(id: string): void {
  el.packDesc.textContent = packs.find((p) => p.id === id)?.description ?? "";
}

/** Reflect a config snapshot into the UI without firing input events. */
function render(cfg: Config): void {
  el.enabled.checked = cfg.enabled;
  el.mouse.checked = cfg.mouse_enabled;
  el.pack.value = cfg.pack;
  renderPackDesc(cfg.pack);
  el.volume.value = String(Math.round(cfg.volume * 100));
  el.volumeOut.textContent = pct(cfg.volume);
  el.pitch.value = String(Math.round(cfg.pitch_variation * 100));
  el.pitchOut.textContent = pct(cfg.pitch_variation);
  el.release.checked = cfg.play_on_release;
}

async function init(): Promise<void> {
  try {
    packs = await invoke<PackInfo[]>("list_packs");
    for (const pack of packs) {
      const option = document.createElement("option");
      option.value = pack.id;
      option.textContent = pack.name;
      el.pack.appendChild(option);
    }
    render(await invoke<Config>("get_config"));
  } catch (error: unknown) {
    el.hint.textContent = "Could not load settings.";
  }

  // Keep the UI in sync when settings change elsewhere (e.g. the tray menu).
  await listen<Config>("config", (event) => render(event.payload));

  el.enabled.addEventListener("change", () => {
    void invoke("set_enabled", { enabled: el.enabled.checked });
  });

  el.mouse.addEventListener("change", () => {
    void invoke("set_mouse_enabled", { enabled: el.mouse.checked });
  });

  el.pack.addEventListener("change", () => {
    renderPackDesc(el.pack.value);
    void invoke("set_pack", { pack: el.pack.value }).then(() =>
      invoke("play_test"),
    );
  });

  el.release.addEventListener("change", () => {
    void invoke("set_play_on_release", { play: el.release.checked });
  });

  el.volume.addEventListener("input", () => {
    const value = Number(el.volume.value) / 100;
    el.volumeOut.textContent = pct(value);
    void invoke("set_volume", { volume: value });
  });

  el.pitch.addEventListener("input", () => {
    const value = Number(el.pitch.value) / 100;
    el.pitchOut.textContent = pct(value);
    void invoke("set_pitch", { pitch: value });
  });

  el.test.addEventListener("click", () => {
    void invoke("play_test");
  });
}

window.addEventListener("DOMContentLoaded", () => {
  void init();
});
