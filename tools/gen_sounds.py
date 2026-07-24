#!/usr/bin/env python3
"""Procedurally synthesize ORIGINAL keyboard/mouse sound packs.

All samples are generated from math (filtered noise + resonant decaying
bodies). No third-party audio is used, so the output is original work
releasable under CC0.

Each pack gets 10 WAVs (16-bit mono 44.1kHz) plus a generated pack.json.
Stdlib only (wave, struct, math, random, json).

Usage: gen_sounds.py <sounds-root-dir> [pack ...]
"""
import json
import math
import os
import random
import struct
import sys
import wave

SR = 44100


def _one_pole_lowpass(samples: list[float], cutoff_hz: float) -> list[float]:
    """Simple one-pole low-pass to tame harsh noise."""
    dt = 1.0 / SR
    rc = 1.0 / (2 * math.pi * cutoff_hz)
    alpha = dt / (rc + dt)
    out = []
    prev = 0.0
    for s in samples:
        prev = prev + alpha * (s - prev)
        out.append(prev)
    return out


def synth_click(
    dur: float = 0.045,
    body_hz: float = 180.0,
    body_decay: float = 60.0,
    click_decay: float = 320.0,
    click_cut: float = 6500.0,
    body_gain: float = 0.55,
    click_gain: float = 0.9,
    ring_hz: float = 0.0,
    ring_gain: float = 0.0,
    ring_decay: float = 25.0,
    amp: float = 0.85,
    seed: int = 0,
) -> list[float]:
    """Synthesize a single percussive click.

    - a short broadband 'click' transient (filtered noise, very fast decay)
    - a resonant 'body' (decaying sine) giving the low 'thock'
    - an optional metallic 'ring' overtone (e.g. typewriter-style)
    """
    rnd = random.Random(seed)
    n = int(SR * dur)

    noise = _one_pole_lowpass([rnd.uniform(-1.0, 1.0) for _ in range(n)], click_cut)

    out = []
    peak = 1e-9
    for i in range(n):
        t = i / SR
        click_env = math.exp(-click_decay * t)
        body_env = math.exp(-body_decay * t)
        # tiny pitch drop on the body for a natural 'thock'
        f = body_hz * (1.0 + 0.15 * math.exp(-90.0 * t))
        s = click_gain * noise[i] * click_env
        s += body_gain * math.sin(2 * math.pi * f * t) * body_env
        if ring_gain > 0.0:
            s += ring_gain * math.sin(2 * math.pi * ring_hz * t) * math.exp(-ring_decay * t)
        out.append(s)
        peak = max(peak, abs(s))

    # normalize then apply target amplitude, with a 2ms fade-out to avoid clicks
    fade = int(SR * 0.002)
    norm = amp / peak
    samples = []
    for i, s in enumerate(out):
        v = s * norm
        if i >= n - fade:
            v *= (n - i) / fade
        samples.append(v)
    return samples


def write_wav(path: str, samples: list[float]) -> None:
    with wave.open(path, "w") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        frames = bytearray()
        for s in samples:
            v = int(max(-1.0, min(1.0, s)) * 32767)
            frames += struct.pack("<h", v)
        w.writeframes(bytes(frames))


def _spec(base: dict, **overrides) -> dict:
    merged = dict(base)
    merged.update(overrides)
    return merged


def pack_specs(character: dict) -> dict[str, dict]:
    """Per-file synth parameters for one pack character.

    `character` provides the base timbre; per-key overrides shape Space/Enter/
    modifiers/mouse relative to it.
    """
    c = character
    base = dict(
        dur=c["dur"],
        body_hz=c["body_hz"],
        body_decay=c["body_decay"],
        click_decay=c["click_decay"],
        click_cut=c["click_cut"],
        body_gain=c["body_gain"],
        click_gain=c["click_gain"],
        ring_hz=c.get("ring_hz", 0.0),
        ring_gain=c.get("ring_gain", 0.0),
        ring_decay=c.get("ring_decay", 25.0),
    )
    rel = dict(
        dur=base["dur"] * 0.65,
        body_decay=base["body_decay"] * 1.6,
        click_gain=base["click_gain"] * 0.55,
        body_gain=base["body_gain"] * 0.6,
        ring_gain=0.0,
    )
    return {
        "key_press.wav": _spec(base, amp=0.80, seed=c["seed"] + 1),
        "key_release.wav": _spec(base, **rel, amp=0.45, seed=c["seed"] + 2),
        "space_press.wav": _spec(
            base,
            dur=base["dur"] * 1.4,
            body_hz=base["body_hz"] * 0.65,
            body_decay=base["body_decay"] * 0.75,
            body_gain=min(1.0, base["body_gain"] * 1.25),
            amp=0.90,
            seed=c["seed"] + 3,
        ),
        "space_release.wav": _spec(
            base, **rel, body_hz=base["body_hz"] * 0.8, amp=0.50, seed=c["seed"] + 4
        ),
        "enter_press.wav": _spec(
            base,
            dur=base["dur"] * 1.2,
            body_hz=base["body_hz"] * 0.8,
            amp=0.88,
            seed=c["seed"] + 5,
        ),
        "enter_release.wav": _spec(base, **rel, amp=0.48, seed=c["seed"] + 6),
        "backspace_press.wav": _spec(
            base, body_hz=base["body_hz"] * 1.1, amp=0.82, seed=c["seed"] + 7
        ),
        "modifier_press.wav": _spec(
            base, body_hz=base["body_hz"] * 0.75, amp=0.78, seed=c["seed"] + 8
        ),
        # Mouse: much shorter and brighter — a subtle 'tick'.
        "mouse_press.wav": _spec(
            base,
            dur=0.022,
            body_hz=base["body_hz"] * 2.2,
            body_decay=300.0,
            click_decay=600.0,
            click_cut=min(9000.0, base["click_cut"] * 1.3),
            body_gain=0.3,
            click_gain=1.0,
            ring_gain=0.0,
            amp=0.6,
            seed=c["seed"] + 9,
        ),
        "mouse_release.wav": _spec(
            base,
            dur=0.018,
            body_hz=base["body_hz"] * 2.6,
            body_decay=380.0,
            click_decay=700.0,
            click_cut=min(9000.0, base["click_cut"] * 1.3),
            body_gain=0.22,
            click_gain=0.9,
            ring_gain=0.0,
            amp=0.45,
            seed=c["seed"] + 10,
        ),
    }


# Four distinct switch characters, all purely synthetic.
PACKS = {
    "thock": dict(
        name="Thock",
        description="Deep, muted 'thocky' switches.",
        dur=0.045,
        body_hz=185.0,
        body_decay=70.0,
        click_decay=320.0,
        click_cut=6500.0,
        body_gain=0.55,
        click_gain=0.9,
        seed=100,
    ),
    "clicky": dict(
        name="Clicky",
        description="Bright, sharp clicky switches.",
        dur=0.040,
        body_hz=340.0,
        body_decay=110.0,
        click_decay=480.0,
        click_cut=9000.0,
        body_gain=0.35,
        click_gain=1.0,
        ring_hz=2400.0,
        ring_gain=0.12,
        ring_decay=180.0,
        seed=200,
    ),
    "tactile": dict(
        name="Tactile",
        description="Medium, rounded tactile switches.",
        dur=0.042,
        body_hz=250.0,
        body_decay=85.0,
        click_decay=380.0,
        click_cut=7500.0,
        body_gain=0.5,
        click_gain=0.85,
        seed=300,
    ),
    "typewriter": dict(
        name="Typewriter",
        description="Vintage typewriter with a metallic ring.",
        dur=0.060,
        body_hz=140.0,
        body_decay=55.0,
        click_decay=300.0,
        click_cut=7000.0,
        body_gain=0.6,
        click_gain=0.95,
        ring_hz=1500.0,
        ring_gain=0.18,
        ring_decay=60.0,
        seed=400,
    ),
}

MANIFEST_KEYS = {
    "Space": {"press": "space_press.wav", "release": "space_release.wav"},
    "Enter": {"press": "enter_press.wav", "release": "enter_release.wav"},
    "Backspace": {"press": "backspace_press.wav"},
    "Shift": {"press": "modifier_press.wav"},
    "Control": {"press": "modifier_press.wav"},
    "Alt": {"press": "modifier_press.wav"},
    "Meta": {"press": "modifier_press.wav"},
    "Tab": {"press": "modifier_press.wav"},
    "MouseLeft": {"press": "mouse_press.wav", "release": "mouse_release.wav"},
    "MouseRight": {"press": "mouse_press.wav", "release": "mouse_release.wav"},
    "MouseMiddle": {"press": "mouse_press.wav", "release": "mouse_release.wav"},
}


def write_manifest(out_dir: str, character: dict) -> None:
    manifest = {
        "name": character["name"],
        "author": "Thock project",
        "license": "CC0-1.0",
        "version": "1.0.0",
        "description": character["description"]
        + " Procedurally synthesized; original work, no third-party audio.",
        "default": {"press": "key_press.wav", "release": "key_release.wav"},
        "keys": MANIFEST_KEYS,
    }
    with open(os.path.join(out_dir, "pack.json"), "w") as f:
        json.dump(manifest, f, indent=2)
        f.write("\n")


def main() -> None:
    root = sys.argv[1]
    wanted = sys.argv[2:] or list(PACKS)
    for pack_id in wanted:
        character = PACKS[pack_id]
        out_dir = os.path.join(root, pack_id)
        os.makedirs(out_dir, exist_ok=True)
        for filename, kw in pack_specs(character).items():
            write_wav(os.path.join(out_dir, filename), synth_click(**kw))
        write_manifest(out_dir, character)
        print(f"pack '{pack_id}': 10 wavs + pack.json")


if __name__ == "__main__":
    main()
