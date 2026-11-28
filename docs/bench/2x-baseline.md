# Banco de performance 2.x — cómo re-ejecutarlo

Un solo comando mide la build de 2.x que está instalada y escribe una **baseline**; otro comando
vuelve a medir y compara contra esa baseline. La salida es fps mediana, p99 y el veredicto
(mejora / sin cambio / regresión), así que un cambio futuro se juzga contra este número y no contra
otra corrida del mismo día.

## Requisitos

- Teléfono Android conectado por `adb` (`ANDROID_SERIAL`, por defecto `R5CW60624NE`).
- App 2.x instalada y **ya dentro de la escena**, con el horario fijo que se quiera medir
  (`Dynamic Skybox` OFF + `Custom Skybox = Midday`), perfil `High` y `Dynamic Graphics` OFF.
- `perfetto` en el device (viene en Android) y `trace_processor_shell` en `.bench-tmp/`.
- `Thermal Status` del teléfono ≤ 1; si está más caliente, el banco espera solo.

## Correr una baseline

```bash
# 1. dejar la app en la escena y en la condición (una vez, a mano o por deeplink)
# 2. medir 6 ventanas de 30 s y escribir la baseline
python3 scripts/bench/2x-vs-1x/baseline.py run \
  --out docs/bench/data/2x-vs-1x-3206/baselines/2x-2.0.0.json \
  --scene bafkreicbvebg6jla42rgk7adk7g7ksrtonbhigcfhfdwdvntkm3hs3d44e \
  --position 2,3 --profile 3 --windows 6
```

Cada ventana escribe su propio directorio con `frames.csv`, `meta.json`, `state.json` y la captura
de Perfetto. `state.json` guarda el perfil del engine, `scene_id`, `position`, `Thermal Status`, y
las dos fuentes de fps (frame timeline y contador del engine); si algo no coincide, la ventana queda
descartada con el motivo en `INVALID.md`.

## Comparar una build posterior

```bash
python3 scripts/bench/2x-vs-1x/baseline.py compare \
  --baseline docs/bench/data/2x-vs-1x-3206/baselines/2x-2.0.0.json \
  --windows 6 --json /tmp/report.json
```

Devuelve:

```json
{
  "baseline": {"version": "2.0.0", "median": 39.28, "runs": [...]},
  "current":  {"runs": [39.58, 39.4, 42.94]},
  "delta_fps": 0.3, "delta_pct": 0.8,
  "ci95_delta_fps": [-0.04, 3.67],
  "mwu_p_exact": 0.2,
  "verdict": "no change"
}
```

El veredicto sale del intervalo bootstrap: si el IC95 de la diferencia cruza cero, es **sin cambio**.
Con 6 vs 6 el piso del test exacto es 0,0022: no se puede pedir más con este diseño.

## Qué no hace

- No cambia la condición: perfil y horario se dejan como estén y se **verifican** por ventana.
- No mide iOS: para eso está `run_ios_windows.sh` + `ios_frames.py` (`xctrace`, Game Performance).
- No compara contra 1.x: para eso está `compare.py` con los datos de
  `docs/bench/data/2x-vs-1x-3206/android-a54/`.
