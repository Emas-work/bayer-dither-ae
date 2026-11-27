# Bayer Dither – plugin do After Effects

Ordered dithering (Bayer 2×2 / 4×4 / 8×8 / 16×16) dla After Effects, macOS i Windows. 8, 16 i 32 bpc.

## Skąd wziąć gotowy plugin

Zakładka **Actions** → ostatni zielony przebieg „Build plugin” → na dole sekcja **Artifacts**:
- `BayerDither_Mac` – `Bayer Dither.plugin` (Apple Silicon + Intel)
- `BayerDither_Windows` – `Bayer Dither.aex`

## Instalacja – macOS

1. Rozpakuj, aż zobaczysz `Bayer Dither.plugin`.
2. Skopiuj do `/Library/Application Support/Adobe/Common/Plug-ins/7.0/MediaCore/`
3. Plik z internetu dostaje blokadę kwarantanny – zdejmij ją w Terminalu:
   ```
   sudo xattr -dr com.apple.quarantine "/Library/Application Support/Adobe/Common/Plug-ins/7.0/MediaCore/Bayer Dither.plugin"
   ```
4. Uruchom AE → **Effect › Plumcat › Bayer Dither**.

## Instalacja – Windows

1. Skopiuj `Bayer Dither.aex` do `C:\Program Files\Adobe\Common\Plug-ins\7.0\MediaCore\`
2. Prawy klik → Właściwości → „Odblokuj” → OK.
3. Uruchom AE → **Effect › Plumcat › Bayer Dither**.

## Parametry

| Parametr | Opis |
|---|---|
| Style | Bayer 2×2 / 4×4 / 8×8 / 16×16 |
| Size | wielkość piksela ditheru (skaluje się przy podglądzie Half/Quarter) |
| Levels | poziomy na kanał (2 = 1-bit) |
| Brightness / Contrast | korekta przed ditherem |
| Mono | dither z luminancji |
| Mono color / Mono opacity | kolor i krycie zapalonych pikseli |
| Mono background | kolor tła w trybie mono |
| Transparent background | w trybie mono tło przezroczyste |
