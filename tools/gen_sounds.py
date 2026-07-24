#!/usr/bin/env python3
"""Procedurally synthesize an ORIGINAL mechanical-keyboard sound pack.

All samples are generated from math (filtered noise + a short resonant
'thock' body with a fast exponential decay). No third-party audio is used,
so the output is original work releasable under CC0.

Output: 16-bit mono 44.1kHz WAV files into the given directory.
Stdlib only (wave, struct, math, random).
"""
import math
import os
import random
import struct
import sys
import wave

SR = 44100


def _one_pole_lowpass(samples, cutoff_hz):
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
    dur=0.045,
    body_hz=180.0,
    body_decay=60.0,
    click_decay=320.0,
    click_cut=6500.0,
    body_gain=0.55,
    click_gain=0.9,
    amp=0.85,
    seed=0,
):
    """Synthesize a single percussive key click.

    - a short broadband 'click' transient (filtered noise, very fast decay)
    - a resonant 'body' (decaying sine) giving the low 'thock'
    """
    rnd = random.Random(seed)
    n = int(SR * dur)

    raw_noise = [rnd.uniform(-1.0, 1.0) for _ in range(n)]
    noise = _one_pole_lowpass(raw_noise, click_cut)

    out = []
    peak = 1e-9
    for i in range(n):
        t = i / SR
        click_env = math.exp(-click_decay * t)
        body_env = math.exp(-body_decay * t)
        # tiny pitch drop on the body for a natural 'thock'
        f = body_hz * (1.0 + 0.15 * math.exp(-90.0 * t))
        body = math.sin(2 * math.pi * f * t) * body_env
        s = click_gain * noise[i] * click_env + body_gain * body
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


def write_wav(path, samples):
    with wave.open(path, "w") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        frames = bytearray()
        for s in samples:
            v = int(max(-1.0, min(1.0, s)) * 32767)
            frames += struct.pack("<h", v)
        w.writeframes(bytes(frames))


def main():
    out_dir = sys.argv[1]
    os.makedirs(out_dir, exist_ok=True)

    # (filename, kwargs) — distinct character per key group
    specs = {
        "key_press.wav":       dict(dur=0.045, body_hz=185, body_decay=70,  amp=0.80, seed=1),
        "key_release.wav":     dict(dur=0.030, body_hz=220, body_decay=110, click_gain=0.5, body_gain=0.35, amp=0.45, seed=2),
        "space_press.wav":     dict(dur=0.065, body_hz=120, body_decay=48,  body_gain=0.7, amp=0.90, seed=3),
        "space_release.wav":   dict(dur=0.038, body_hz=150, body_decay=90,  click_gain=0.5, body_gain=0.4, amp=0.50, seed=4),
        "enter_press.wav":     dict(dur=0.055, body_hz=150, body_decay=58,  amp=0.88, seed=5),
        "enter_release.wav":   dict(dur=0.032, body_hz=190, body_decay=100, click_gain=0.5, body_gain=0.4, amp=0.48, seed=6),
        "backspace_press.wav": dict(dur=0.045, body_hz=205, body_decay=75,  amp=0.82, seed=7),
        "modifier_press.wav":  dict(dur=0.050, body_hz=140, body_decay=55,  amp=0.78, seed=8),
    }

    for name, kw in specs.items():
        write_wav(os.path.join(out_dir, name), synth_click(**kw))
        print("wrote", name)


if __name__ == "__main__":
    main()
