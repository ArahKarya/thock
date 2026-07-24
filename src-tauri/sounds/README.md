# Sound packs

Each pack is a folder with a `pack.json` manifest plus its audio files.

## Manifest format

```json
{
  "name": "Display name",
  "author": "You",
  "license": "CC0-1.0",
  "version": "1.0.0",
  "default": { "press": "generic.wav", "release": "generic_up.wav" },
  "keys": {
    "Space":     { "press": "space.wav", "release": "space_up.wav" },
    "Enter":     { "press": "enter.wav" },
    "Backspace": { "press": "backspace.wav" }
  }
}
```

- `default` is used for any key without a specific entry.
- A missing `release` falls back to the pack `default` release (if any).
- Logical key names currently recognized: `Space`, `Enter`, `Backspace`,
  `Tab`, `Shift`, `Control`, `Alt`, `Meta`. Everything else uses `default`.

## Licensing

All bundled audio must be **original or CC0 / public-domain**. Do not copy
audio, icons, or any assets from proprietary applications.

The bundled `thock/` pack is **procedurally synthesized** by
`tools/gen_sounds.py` (filtered noise + a decaying resonant body), so it is
original work released under CC0-1.0.
