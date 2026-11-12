# 2.x vs 1.x — benchmark medido (#3206)

Medición del cliente 2.x contra el 1.x en el mismo teléfono, misma escena y mismo pose, con el
protocolo de [`2x-vs-1x-3206-protocol.md`](./2x-vs-1x-3206-protocol.md) (comandos exactos y criterios
de invalidación). Datos crudos en [`data/2x-vs-1x-3206/`](./data/2x-vs-1x-3206/).

## TL;DR

Versión navegable de todos estos datos, con las tablas generadas desde los CSV y los archivos crudos
descargables: <https://artifacts.dclregenesislabs.xyz/af8787b0-ec0c-49c0-bc58-3e7728a3b180>
(pide login con email de trabajo la primera vez).

- **fps, condición controlada (mundo fijo + Midday fijo + High + dynamic OFF)**: 1.x **40.22**,
  2.x **38.62** → **−4.0 %**, IC95 [−4.18, +1.83] fps, MWU exacto p = 0.24. Con 6 vs 6 eso es
  **empate dentro del ruido**, no una diferencia demostrada. Un bloque anterior publicó −50.9 % con esta
  misma condición: era un artefacto — el brazo 2.x corría el perfil `Very Low` (cap 18 fps) porque el
  shell no llegó a empujar los ajustes antes del primer frame (§1.3). En Genesis Plaza, donde ningún
  build puede fijar el cielo, el par medido dio **25.78 vs 25.88 fps** y queda como descriptivo.
- **Cola de frame times**: 2.x es peor en los peores frames — **p99 66.9 → 74.9 ms (+11.9 %)**,
  **fps p1 (1 % low) 14.9 → 13.4 (−10.6 %)**. La mediana y el p95 son idénticos (±0.1 %). Es decir:
  misma fluidez media, hitches más profundos en 2.x.
- **Memoria**: 2.x **no** empeora. PSS en mundo 2031 → 1967 MB (−3.2 %), pico 2094 → 1998 MB
  (−4.6 %). CPU media 132.7 % → 119.0 % (−10.3 %), temperatura de funda 43.6 → 42.6 °C.
- **Carga**: 2.x entra al mundo antes. `[RenderReady]` t100 (todo lo que está en vista con pipeline
  compilado) **7.7 s → 1.4 s**; arranque en frío hasta `[RenderReady]` **27.8 s → 19.4 s** (mediana).
- **Entrar y salir 20 veces**: 2.x entra **13.2 % antes** (8.5 s vs 9.8 s) y con **237 MB menos**
  (−12.4 %). Ninguno de los dos fuga: el PSS del ciclo 20 está dentro de la dispersión del ciclo 1.
- **Soak de 71 min (2.x)**: sin throttling térmico (piel 38–40 °C), memoria con deriva suave de
  ~+100 MB en la mediana, sin escalones. El fps de ese run no se pudo leer (buffer de Perfetto
  pequeño), ver §8.
- **UI del shell React Native (sólo tiene lado 2.x)**: tiempo a contenido 0.5–2.3 s por pantalla
  (Discover 2.27 s, Backpack 1.85 s, Profile 1.16 s, Settings ≤ 0.5 s), sin fuga en 20 navegaciones
  (−56 MB) y sin crash con 20 taps en ráfaga. Y volver al shell desde el mundo **sí** es scriptable:
  `BACK` → `LEAVE`, o el chevron del HUD (§6).
- **Presupuesto** del issue: fps ≥ 1.x → **PASS dentro del ruido** con la luz y el contenido
  controlados (38.62 vs 40.22, −4.0 %, p = 0.24); la cola sigue peor en 2.x. RSS ≤ 1.x + 100 MB →
  **PASS** (2.x usa menos).

## Qué se comparó

| | 1.x | 2.x |
|---|---|---|
| versión | `1.16.1` (versionCode 2688) | `2.0.0` (versionCode 2723) |
| rama / commit | `release` @ `cfc24b4e5a1f0bc9f2d30f01f88bfa21a7aae279` | `main` @ `e3b7aa102bd0fbfe3a51cc2204248a59662b3fde` |
| APK | `mobile-artifacts…/android/release/latest/…apk` | `mobile-artifacts…/android/main/e3b7aa102…/…apk` |
| firma | release CI, `CN=Decentraland`, SHA-256 `8e75d23a…c4784` | ídem (mismo firmante) |
| debuggable | no | no |
| HUD | `v1.16.1.2688-cfc24b4-prod` | `2.0.0.2723-e3b7aa1-staging` |

Los dos APKs son `export --release` de CI; no participó ningún build debug ni con profiler.

**Contenido pinneado (idéntico en ambos):** `[RenderReady]` reporta
`scene_id=bafkreig5z6onyvwk4hrklg7einxgspq3nqr54a5zpbnfp53vpkigyatu2a`, `position="0,-2"`,
`profile=0`, `anti_aliasing=0`, `shadow_quality=0`, `bloom_quality=0`, `async_shaders=true`. Es la
misma Genesis Plaza, mismo perfil gráfico y mismo spawn en las dos ramas, que es la condición para
que un Δ% signifique algo.

## Dispositivos

| device | build | estado |
|---|---|---|
| Samsung Galaxy A54 5G (`SM-A546E`, Android 16, arm64-v8a, 120 Hz) | 1.16.1 y 2.0.0 | medido |
| iPhone 16e (`iPhone17,5`, iOS 26) | 2.0.0 (RN) y 1.x-mode (app Godot del mismo commit) | sólo arranque, ver §5 |

## 1. Genesis Plaza — Android A54

5 corridas por build, estrictamente alternadas (`2x 1x 2x 1x …`), 25 s de settle y ventana de 60 s
con la cámara quieta en el spawn. Mediana de las 5 medianas por corrida:

| métrica | 1.x | 2.x | Δ (2.x vs 1.x) |
|---|---:|---:|---:|
| fps medio | 17.9 | 18.1 | +0.7 % |
| fps p5 (5 % low) | 15.0 | 15.0 | −0.1 % |
| fps p1 (1 % low) | 14.9 | 13.4 | **−10.6 %** |
| frame time p50 | 58.3 ms | 58.2 ms | −0.0 % |
| frame time p95 | 66.7 ms | 66.8 ms | +0.1 % |
| frame time p99 | 66.9 ms | 74.9 ms | **+11.9 %** |
| frame time máx | 91.6 ms | 100.1 ms | **+9.3 %** |
| PSS estable (mundo) | 2031 MB | 1967 MB | **−3.2 %** |
| PSS pico | 2094 MB | 1998 MB | **−4.6 %** |
| CPU media (todos los cores) | 132.7 % | 119.0 % | −10.3 % |
| temperatura al cerrar ventana | 43.6 °C | 42.6 °C | −1.0 °C |

Presupuesto: **fps PASS**, **RSS PASS** (umbral 2131 MB, 2.x queda 164 MB por debajo).

Dispersión por corrida (para que se vea que no es un promedio tapando dos poblaciones):

| corrida | fps | fps p1 | PSS | temp | CPU |
|---|---:|---:|---:|---:|---:|
| `1x-gp-r2` | 17.93 | 13.33 | 2031 MB | 43.7 °C | 138.4 % |
| `1x-gp-r4` | 17.85 | 14.95 | 2046 MB | 43.7 °C | 132.4 % |
| `1x-gp-r6` | 17.94 | 14.96 | 2026 MB | 43.9 °C | 118.4 % |
| `1x-gp-r8` | 17.94 | 14.95 | 2021 MB | 43.5 °C | 132.7 % |
| `1x-gp-r10` | 17.94 | 13.33 | 2060 MB | 43.5 °C | 133.4 % |
| `2x-gp-r1` | 18.36 | 13.36 | 1967 MB | 42.7 °C | 116.4 % |
| `2x-gp-r3` | 18.19 | 13.36 | 1953 MB | 42.4 °C | 123.5 % |
| `2x-gp-r5` | 18.07 | 13.33 | 1987 MB | 42.6 °C | 118.9 % |
| `2x-gp-r7` | 17.92 | 14.93 | 2070 MB | 42.7 °C | 119.0 % |
| `2x-gp-r9` | 17.93 | 13.34 | 1959 MB | 42.5 °C | 138.1 % |

**Avatares presentes.** Genesis Plaza estaba poblada en todas las corridas (jugadores reales, no
bots): 5–10 avatares visibles por captura, contados a mano en `screen.png` de cada corrida. No hubo
forma de fijar N (§10 de limitaciones), así que este bloque es "plaza poblada tal cual", no un test de
N avatares.

**Diferencia de overlay a declarar.** En las corridas de 1.x quedó arriba el cartel de primer uso
"Tap the screen to start exploring" con los globos "Character movement"/"Emotes"
(`1x-gp-r4/screen.png`), mientras que en 2.x el tutorial se había descartado en la preparación
(`SKIP`). Son overlays del motor dibujados sobre el mundo: suman UI a 1.x durante la ventana. No
invierte el resultado (2.x es peor en la cola, no mejor), pero es una asimetría real y se declara.

## 1.1 Corrección posterior: dos confusores que invalidaron la matriz HIGH

La primera versión de este reporte publicó una matriz con `HIGH` + dynamic graphics OFF (escenario
`gp-hi-pair/`) que ya no se sostiene. Al re-leer la evidencia de cada corrida aparecieron dos variables
que no estaban controladas, y las dos mueven el resultado más que la diferencia entre builds.

### El perfil no era `HIGH` en todas las corridas

Los ajustes se aplican tapeando el panel, y el panel mostró `High` mientras el engine cargaba otro
preset. La prueba está en el campo `profile` del JSON de `[RenderReady]` de cada corrida (2 = medium,
3 = high):

| corrida | profile cargado | fps |
|---|---:|---:|
| `1x-hi-p1` | **0 (low)** | 24.49 |
| `1x-hi-p2` | 3 | 15.47 |
| `1x-hi-p3` | 3 | 15.92 |
| `1x-hi-p4` | **0 (low)** | 13.50 |
| `2x-hi-p1` | **2 (medium)** | 31.31 |
| `2x-hi-p2` | 3 | 30.53 |

La corrida que más se repitió (24.49 fps, el "outlier" de 1.x que se re-midió cuatro veces) corrió en
**low**, y la mejor de 2.x (31.31 fps) en **medium**. El directorio queda marcado inválido en
`gp-hi-pair/INVALID.md`.

### La hora del mundo no estaba fijada

Con `Dynamic Skybox` encendido el sol avanza, y esta escena cuesta distinto según dónde esté. La
luminancia media de la captura de la ventana (`luma`: media del canal L de `screen.png`) contra fps,
todas las corridas de `gp-hi-pair/`:

| luma | build | fps |
|---:|---|---:|
| 129.4 | 1.x | 24.49 |
| 128.5 | 2.x | 30.53 |
| 117.6 | 2.x | 31.31 |
| 90.3 | 1.x | 15.92 |
| 89.5 | 1.x | 13.50 |
| 78.0 | 1.x | 15.47 |

Con el cielo fijado (`Dynamic Skybox` OFF) el mismo 2.x dio **15.18 / 14.42 / 13.47 fps** (luma 75.5 /
77.6 / 72.6): **2.2× menos que la misma build con el sol alto**. La variable sol mueve el resultado más
que la diferencia entre builds, así que ninguna comparación es válida sin fijarla.

### El ajuste de cielo sí llega al mundo, pero no a 2.x

Se probó en una parcela **sin escena** (`100,100`), que es el único lugar donde el reloj no se lo queda
la escena: en Genesis Plaza `scene_runner/components/skybox_time.rs` pone `sdk_skybox_time_active = true`
y el mundo ignora el ajuste del cliente. Con `Dynamic Skybox` OFF y `Custom Skybox` elegido a mano, en
la misma sesión y con el perfil en `High`:

| build | preset elegido | luma del mundo | fps |
|---|---|---:|---:|
| 1.x | Midnight | 34.4 | — |
| 1.x | Morning | 77.9 | — |
| 1.x | **Midday** | **110.6 / 111.5 / 111.9** | **33.96 / 40.29 / 36.70** |
| 2.x | **Midday** | **33.1** | no se midió |

1.x responde a los tres presets, con una separación de 76 puntos de luma. 2.x no: el panel guarda
`Midday`, el preview pinta cielo de día y tras reiniciar el panel lo sigue mostrando, pero el mundo
renderiza **noche**. La ruta está en el código: `lib/crates/dcl_engine/src/sky/skybox_time.rs` lee

```rust
engine.setting("skybox_time".into()).try_to::<f64>().unwrap_or(0.0)
```

y `0.0` normalizado es medianoche. Un valor que no llega como número se convierte, sin aviso, en la
hora más oscura posible. En 1.x el mismo ajuste sí llega, así que todo el que ponga `Dynamic Skybox` OFF
en 2.x ve noche a cualquier hora: **la matriz pedida (dynamic OFF + HIGH) no se puede medir en 2.x hasta
que esto se arregle**, y arreglarlo no es parte de un PR de medición.

Las capturas que lo prueban están en
`docs/bench/data/2x-vs-1x-3206/android-a54/sky-fixed-midday/evidence/` (`2x-settings-midday-stored.png`
con el panel mostrando Midday, `2x-world-at-midday-setting.png` en noche, `1x-world-at-midday-setting.png`
en día), y el resumen en `sky-fixed-midday/RESULT.json`.

### Qué sí sale de la niebla: la luz mueve el resultado más que el build

Aunque el sol no se pueda fijar, su efecto se midió. Mismo build, mismo perfil, misma temperatura:
**31.3 fps con el sol alto contra 13.5–15.2 fps con el sol bajo**, es decir **2.2×**, más que cualquier
diferencia entre 1.x y 2.x. Doce ventanas de 30 s en una sola sesión de 2.x, espaciadas 111 s, dieron
luma 56.4–61.0 (fps 18.4–22.9, mediana 21.5, p50 50 ms, CPU ~190 %): dentro de una sesión la luz es
estable, así que esa banda es una condición reproducible.

### La matriz HIGH en Genesis Plaza no se puede parear

Con `HIGH` + dynamic OFF en la plaza, la misma etiqueta de ajuste produce ventanas con luma
sistemáticamente distinta por build, y el fps la sigue:

| build | corridas | fps | luma | p50 | CPU |
|---|---|---:|---|---:|---:|
| 1.x | `1x-v3-r3` | 14.06 | 86.0 | 67.0 | 170.7 % |
| 1.x | `1x-v3-r1` | 26.87 | 101.6 | 33.4 | 182.2 % |
| 1.x | `1x-v3-r2` | 15.83 | 117.6 | 58.4 | 181.7 % |
| 2.x | `2x-drift` s1–s12 | 21.5 (mediana) | 56.4–61.0 | 50.0 | ~190 % |
| 2.x | `2x-sky-r1..r3` | 15.2 / 14.4 / 13.5 | 75.5 / 77.6 / 72.6 | 66.7–75.0 | 133–149 % |

1.x nunca baja de luma 86 en estas corridas y 2.x nunca sube de 78 con el mismo ajuste, así que no
existe un par comparable. **No se publica un Δ % de esa matriz.**

### Qué cambió en el método

- Guard nuevo: la corrida se acepta sólo si el `[RenderReady]` **de su propio pid** trae
  `"profile":3` y no aparece ninguna línea `Profile (downgrade|upgrade)` dentro de la ventana. El guard
  ya abortó tres corridas 1.x que habían arrancado en profile 0.
- `logcat -c` antes de cada lanzamiento: antes el buffer traía líneas de lanzamientos anteriores y una
  corrida quedó "verificada" con la evidencia de otra.
- Se descarta el tutorial de primer uso (`Welcome to Decentraland`): es un modal que oscurece la
  ventana entera y hundió una corrida a luma 48.9.
- Matriz pareada: se alternan builds con reinstalación por corrida (el cambio de buildCode obliga a
  desinstalar), `state.json` por corrida y comparación sólo entre corridas de luma cercana.
- Cielo: se mide en la parcela `100,100` cuando se quiere el reloj fijo (en Genesis Plaza manda la
  escena) y las ventanas de una condición se toman en una sola sesión, sin relanzar.

**Lo que sigue válido:** §1 (5 + 5 corridas con los defaults que se entregan, dynamic graphics ON)
no se toca. Con el cap encendido los dos builds quedan clavados en ~18 fps y el sol no mueve la aguja;
lo que queda invalidado es la matriz `HIGH` + dynamic OFF de `gp-hi-pair/`.

**iOS quedó medible.** El HUD del engine está en el árbol de accesibilidad: `/Users/lordmanuel/bin/iphone ui`
devuelve `2.0.0.2723-e3b7aa1-staging - 60 FPS` (muestreo a 1 Hz; primeras muestras 59–60 fps en Bloom
Garden con los ajustes que se entregan). Además los ajustes de iOS son scriptables: `Documents/settings.json`
del contenedor de la app se lee y se escribe con `xcrun devicectl device copy from|to`, con las claves
`dynamic_graphics_enabled`, `dynamic_skybox`, `skybox_time` y `graphic_profile`.

## 1.2 Genesis Plaza, segunda pasada: perfil verificado y el sol como variable medida

Se rehizo el bloque `HIGH` con las dos condiciones que faltaban: el perfil se lee del `[RenderReady]`
**del pid de esa corrida** (las 15 ventanas nuevas arrancaron en `"profile":3`, sin una sola línea
`Profile (downgrade|upgrade)` dentro de la ventana) y la compuerta térmica cierra antes de cada ventana
(AP ≤ 38 °C, piso de 480 s entre ventanas). Dispositivo: A54, pantalla pinchada a 120 Hz, perfil HIGH,
dynamic graphics OFF, ventana de 30 s, métrica primaria `fps_avg` (media de los intervalos entre frames).

### El hallazgo que ordena todo: la hora del juego mueve el resultado más que la versión

Genesis Plaza **no fija el sol**. La escena no publica una hora fija, así que el cielo sigue el reloj
del mundo y el costo de la escena cambia con él. Nueve ventanas de 1.x y seis de 2.x, ordenadas por
hora del juego (que se calcula del reloj UTC, `(utc × 12) mod 86400`):

| hora del juego | build | fps_avg | p50 (ms) |
|---|---|---:|---:|
| 10:13 | 1.x | 27.64 | 33.4 |
| 10:35 | 1.x | 26.46 | 33.5 |
| 10:54 | 1.x | 24.55 | 41.7 |
| 11:16 | 1.x | 25.15 | 41.6 |
| 11:38 | 1.x | 26.13 | 41.5 |
| 12:01 | 1.x | 25.81 | 41.6 |
| 14:59 | 2.x | 25.62 | 41.6 |
| 15:26 | 2.x | 26.52 | 41.6 |
| 15:52 | 2.x | 26.38 | 41.6 |
| 16:19 | 2.x | 25.75 | 41.6 |
| 16:50 | 2.x | 26.49 | 41.6 |
| 17:17 | 2.x | 25.25 | 41.6 |
| 18:42 | 1.x | 22.61 | 41.5 |
| 19:12 | 1.x | **18.26** | 50.0 |
| 19:39 | 1.x | 19.06 | 50.0 |

Del mediodía al atardecer la **misma build pierde 34 % de fps** (27.64 → 18.26) y el `p50` salta de
4 períodos de vsync (33.4 ms) a 6 (50.0 ms). Ese salto no es ruido térmico: las tres corridas de las
18:42–19:39 pasaron por la misma compuerta que las del mediodía, con AP ≤ 38.7 °C al abrir la ventana.

**Consecuencia para el reporte:** las primeras seis ventanas de cada build no estaban pareadas (1.x de
10:13 a 12:01, 2.x de 14:59 a 17:17), así que su igualdad de medianas (25.78 contra 25.88) es una
coincidencia, no un resultado. Esa tabla no se usa para publicar un Δ %.

### Cómo se parean ahora

Cada build corre la **misma lista de horas del juego**, y las ventanas se emparejan por esa hora. El
reloj del mundo es función pura de UTC con período 7200 s, así que la hora del juego se puede esperar:
`utc_phase.py --wait-until <fase>` duerme hasta que la fase cae en la ventana de ±2 s de juego
(±0.17 s reales) y recién ahí abre la ventana de medición. La comparación se hace por pares
(`matched.py`: diferencia por par, mediana de las diferencias, IC bootstrap y test de signos exacto),
que es la prueba correcta para datos pareados y no exige normalidad.

### El cielo no es el mismo entre builds, aunque la hora sí

Mismo escenario, mismos ajustes, misma hora: la región del cielo (25 % superior de la captura) sale
más brillante en 2.x.

| build | luma del cielo (25 % superior), 6 ventanas | luma del cielo (15 % central superior) |
|---|---:|---:|
| 1.x | 117.0–124.9 | 90.6–107.7 |
| 2.x | 134.7–139.5 | 118.8–131.5 |

Son ~15–20 puntos de diferencia sostenida. La luma del recuadro central **no** sirve como control de
luz en esta escena (los avatares de la plaza entran y salen del recuadro: la misma sesión dio 91.8 y
88.4 en dos minutos), por eso el pareo se hace por hora del juego y no por luminancia. Queda declarado
como diferencia de render entre builds, no como diferencia de hora.

### iOS (iPhone 16e): 2.x medido, 1.x bloqueado por firma

Siete sesiones de 20 muestras a 1 Hz del contador del HUD, en Genesis Plaza, con el build
`2.0.0.2723-e3b7aa1-staging`:

| sesión | mediana | mín | máx |
|---|---:|---:|---:|
| r1–r7 | 40.0 / 35.5 / 37.5 / 39.0 / 36.0 / 36.0 / 36.0 | 29 | 47 |

Mediana de las medianas **36.0 fps** sobre 140 muestras. iOS no expone frame timeline por frame a
terceros (Android sí, vía SurfaceFlinger/Perfetto), así que la métrica es un promedio del HUD y **no
hay p99** en iOS; se probó `xctrace` con las plantillas `Game Performance` y `Animation Hitches` sobre
el proceso `DclRn` y no producen tabla de present por frame.

El 1.x de iOS **no se pudo medir todavía**: el ipa local (`Decentraland.ipa`, versión 1.1 build 27) está
firmado con un perfil de aprovisionamiento **vencido**, y el perfil válido que existe para ese bundle id
pertenece al team `8T73XM973P`, cuyo certificado no está en este Mac. El error exacto está en
`docs/bench/data/2x-vs-1x-3206/ios-16e/IOS-TRACK.md`. La salida es re-firmar ese ipa con un team
propio y otro bundle id, o compilar `release-1.16.2` para iOS; ambas están en curso y este párrafo se
actualiza con el resultado.

### Mediodía fijo: el arreglo que hace medible la condición pedida

La condición original de este benchmark —skybox y horario fijos— **no se puede cumplir en 2.x sin tocar
el código**, porque el preset nunca llega al mundo. Se arregló con una línea real en
`lib/crates/dcl_engine/src/sky/skybox_time.rs`: `skybox_time` es `SettingKind::Int`, el store devuelve
`Variant::Int` y el lector pedía `try_to::<f64>()` estricto; ahora acepta los dos tipos numéricos, igual
que ya hacía `world::explorer::setting_f32`. Con el build de ese commit, misma parcela (`100,100`) y
mismo ajuste (`Dynamic Skybox` OFF + `Custom Skybox: Midday`):

| build | luma del mundo | qué se ve |
|---|---:|---|
| 2.x sin parche | 33.1 | medianoche |
| 2.x **con parche** | **100.2** | mediodía |

Es un build parcheado, no el release, y el diff completo es una función de lectura de ajuste (nada de
render). Se declara como tal en cada tabla que lo use.

### El número final: mediodía fijo, mismo contenido, mismo preset

Condición medida, idéntica en los dos brazos:

- **mismo mundo**: `monsterrecon.dcl.eth`, entrado desde la place card de Discover en ambos builds. No se
  usó la parcela `100,100` como coordenada porque los dos builds resuelven contenido distinto ahí (1.x
  renderiza terreno vacío, 2.x carga el world).
- **misma luz**: `Dynamic Skybox` OFF + `Custom Skybox: Midday`, verificado en cada ventana por la luma
  del recuadro central (1.x 97.6–99.9, 2.x High 96.9–99.8). La luma no es el control, es la prueba de que
  el control se aplicó: en 2.x sólo llega con el parche de `skybox_time`.
- **mismo preset**: `High` en los dos builds, `Dynamic Graphics` OFF, y **el preset verificado por su
  efecto**, no por el panel: en 2.x Medium pega el cap de 30 fps del preset (29.96 medido) y High lo
  supera, así que el perfil está aplicado de verdad.
- **misma térmica**: compuerta por `Thermal Status` de la plataforma (0 en los dos brazos, AP 38–46 °C
  sin throttling), 6 ventanas de 30 s por brazo contra la misma instancia viva.

| build | ventanas | fps mediana | rango | luz por ventana |
|---|---:|---:|---|---|
| 1.x release, High | 6 | **40.22** | 37.96–41.77 | 97.6–99.9 |
| 2.x parcheado, High | 6 | **38.62** | 36.15–41.31 | 96.9–99.8 |

**Δ = −1.60 fps (−4.0 %)**. IC95 bootstrap de la diferencia de medianas **[−4.18, +1.83] fps**;
Mann-Whitney exacto **p = 0.2403**. Con 6 vs 6 esto **no es una diferencia demostrada**: el intervalo
cruza cero y el p está lejos del piso de 0.0022 que este diseño puede alcanzar. Lo que sí queda del lado
de 2.x es la cola (§1): p99 +11.9 %, fps p1 −10.6 %, gastando 10.3 % menos CPU.

#### El −50.9 % anterior era un artefacto, y cómo se detectó

El primer bloque de esta misma condición dio 19.77 fps en 2.x y se publicó como −50.9 %. Era falso: esas
ventanas corrían con el perfil **`Very Low`, cuyo preset fija `limit_fps = Fps18`** (18 fps). La cadena:

- el shell 2.x lanza el engine **antes** de empujar los ajustes guardados, así que el primer
  `[RenderReady]` trae `"profile":0` (Very Low, built-in) y el motor se queda ahí;
- el panel mostraba `High` y `Midday`, y la luma (104.4–105.0) sí probaba el `Midday` —el parche de
  `skybox_time` está en el engine y se aplica cuando el setting llega—, pero **no probaba el preset**;
- 19.77 fps es, aritméticamente, el cap de 18 del `Very Low` + frames sueltos que se cuelan, no un coste
  de render.

Lo que lo destapó fue medir el escalón de preset en la misma sesión y la misma cámara: **Medium = 29.96 fps
(cap de 30 del preset, aplicado)** y **High = 38.62**, contra 19.77 del bloque anterior. El engine lo
confirma en su propio log: `frame sync: frames=14400 fps=41 … "Monster Recon"`.

Consecuencia para el método: **verificar el preset por su efecto medible (cap de fps o tiempo de frame),
nunca por el panel ni por una línea de log de arranque**. La luma sigue sirviendo para la luz y sólo para
la luz.

Un confusor declarado sigue en pie: el brazo 2.x es un **build parcheado** (una función de lectura de
setting, cero código de render). El preset es equivalente entre ramas: `graphic_settings.gd` de
`release-1.16.2` y `PROFILES[3]` de 2.x tienen los mismos valores (shadow 1, bloom 2, scale 1.0,
view_distance 300, dcl_lights true, dcl_light_shadows true, dcl_max_lights 8, aa 0) y `project.godot`
coincide en SSAO/glow/sombras.

Los bloques anteriores de Genesis Plaza con la hora del juego sin fijar (25.78 vs 25.88 fps) quedan como
medición **descriptiva** del estado en que estaban los builds antes del parche: en GP ningún build puede
fijar el cielo, así que ese par no aísla la versión.

**iOS (iPhone 16e), 2.x:** siete sesiones de 20 muestras a 1 Hz, medianas 40.0 / 35.5 / 37.5 / 39.0 /
36.0 / 36.0 / 36.0; mediana de medianas **36.0 fps**. Sin p99: iOS no expone un registro de present por
frame (ver §1.2). El 1.x de iOS no instaló: perfil de aprovisionamiento vencido y sin certificado del
team dueño del bundle id; error textual en `docs/bench/data/2x-vs-1x-3206/ios-16e/IOS-TRACK.md`.

## 2. Carga y arranque

| métrica | 1.x | 2.x | Δ |
|---|---:|---:|---:|
| `am start -W` TotalTime (mediana) | 335 ms | 316 ms | −5.7 % |
| frío → `[RenderReady]` (mediana) | 27 775 ms | 19 433 ms | **−30.0 %** |
| frío → `[RenderReady]` (rango) | 27.4–27.8 s | 15.3–33.0 s | — |
| `t90_ms` (90 % de lo visible dibujado) | 6965 ms | 1410 ms | **−79.8 %** |
| `t100_ms` (100 % dibujado) | 7710 ms | 1410 ms | **−81.7 %** |

El rango del arranque en frío en 2.x es ancho (15.3–33.0 s) porque la alternancia
instalar/desinstalar deja cada build con distinta caché de contenido; la mediana es la comparable y
la comparación es 1 a 1 en número de corridas.

`t90`/`t100` salen del propio `[RenderReady]` que loguea el cliente, con el mismo esquema en ambas
ramas: en 2.x el pipeline asíncrono termina antes porque el port a Rust compila los shaders en
paralelo (`pipelines_mesh`, `pipelines_surface` en el JSON muestran menos pipelines pendientes al
entrar).

## 3. Memoria por estado (Android A54)

1.x, 8 muestras de `dumpsys meminfo` cada 2 s por estado (`states.csv` con los 40 valores crudos,
`states-median.csv` con el resumen):

| estado | PSS mediana | rango |
|---|---:|---:|
| menús (sin entrar al mundo) | 1532 MB | 1532–1533 |
| en mundo (GP) | 2024 MB | 1991–2071 |
| "tras volver a menús" | 2065 MB | 2010–2114 |
| segundo ciclo en mundo | 2065 MB | 2012–2118 |
| "tras la segunda salida" | 2048 MB | 2010–2118 |

`PSS` de mundo coincide con el del bloque de fps (2024 MB vs 2031 MB), lo que valida la muestra.
**Los estados 3 y 5 no son lo que dicen ser**: la secuencia se guionizó con el botón Atrás de Android,
y en 1.x Atrás no abandona el mundo (abre el overlay del tutorial; ver `state-3-after-exit.png`). Lo que
miden las filas 3–5 es "mundo con overlay encima", no la vuelta a menús: el PSS no baja de 2 GB por
que el mundo sigue cargado detrás. Se publican como evidencia de que el mundo no se descarga, no como
un estado de memoria de menús. En 2.x el shell queda detrás del mundo y salir sí es scriptable (§6),
pero este bloque no se repitió para 2.x: la muestra por estado es de 1.x.

## 4. Stress: entrar y salir 20 veces

Ver `data/2x-vs-1x-3206/android-a54/enter-exit/<build>/cycles.csv` (una fila por ciclo: tiempo de
carga, PSS, footprint del motor, `level` del monitor de memoria). Cada ciclo fuerza `am force-stop`
y vuelve a entrar con el mismo deeplink; se muestrea al final de 15 s quieto en el mundo.

| build | ciclos | carga (mediana) | PSS mediana | PSS ciclo 1 → 20 | pendiente PSS |
|---|---:|---:|---:|---:|---:|
| 1.x 1.16.1 | 20 | 9836 ms (9626–15363) | 1917 MB | 2047 → 1889 MB | −4.1 MB/ciclo |
| 2.x 2.0.0 | 20 | 8534 ms (7301–8769) | 1680 MB | 1668 → 1673 MB | +0.1 MB/ciclo |

2.x entra **13.2 % más rápido** a la misma escena en cada ciclo (8.5 s vs 9.8 s de mediana, y el
rango de 2.x es más estrecho) y ocupa **237 MB menos** (−12.4 %).

Ninguno de los dos fuga: el PSS del ciclo 20 queda dentro de la dispersión del ciclo 1 (2.x +5 MB,
pendiente +0.1 MB/ciclo; 1.x incluso baja, −158 MB, pendiente −4.1 MB/ciclo), el tiempo de carga no
se degrada en 20 ciclos y el monitor de memoria del cliente nunca pasa de `level=0`. El ciclo 1 de
1.x es un outlier de carga (15.4 s, caché frío tras instalar) y está incluido en la tabla.

## 5. iOS (iPhone 16e)

Comparación **al mismo commit**: la app React Native 2.0.0 (`main@e3b7aa102`) contra la app Godot del
mismo export (`exports/decentraland-godot-client.xcodeproj`, el modo 1.x). La 1.16.0 que estaba
instalada se perdió al instalar los builds de este benchmark (mismo bundle id, no hay forma de
volver a bajarla sin cuenta de App Store), así que en iOS no se compara 1.16.1 vs 2.0.0 sino
"UI del motor" vs "UI React Native" sobre el mismo motor.

| métrica | 1.x-mode (Godot) | 2.x (React Native) |
|---|---:|---:|
| arranque frío → motor listo | 560 / 1060 / 1560 ms | 550 / 550 / 1059 ms |
| footprint del motor al quedar estable | 3066 / 3098 / 3112 MB | 277 / 279 / 285 MB |

**Sin mundo en iOS en esta pasada**: el iPhone no tiene `adb` equivalente y esta máquina no tiene
inyección de input para iOS 26 (no hay `idb` — la fórmula pide Xcode completo, ni target de
XCUITest), así que no se puede tocar "Play as guest" en el flujo de entrada. Lo publicado es lo que
se puede medir sin input: instalación, arranque y el footprint que el propio motor loguea. Sin
captura de pantalla disponible tampoco se puede afirmar qué pantalla mostraba cada app, por eso las
dos columnas de footprint **no son comparables entre sí** (una app puede estar en su pantalla de
bienvenida y la otra ya en el mundo): se dejan como referencia de arranque, no como resultado.

## 6. UI del shell React Native (2.x, Android A54)

Este bloque sólo tiene lado 2.x: en 1.x las pantallas las dibuja el motor y no existen en el árbol de
accesibilidad, así que no hay nada equivalente que medir. Lo que sigue es cuánto cuesta cada pantalla
del shell RN con el build 2.0.0. Comandos, capturas y método completo en
`data/…/android-a54/ui-2x/README.md`.

**Volver al shell sí es scriptable** — esto corrige una suposición de una versión anterior de este
reporte. Desde el mundo: `KEYCODE_BACK` abre el diálogo React Native `Leave this world?` con
`CANCEL`/`LEAVE` (sus nodos son de RN, `ui.sh tap "LEAVE"` funciona), y el chevron del borde derecho
del HUD (`tap 2283 240` en landscape 2340x1080) entra directo al shell en Backpack. `KEYCODE_ESCAPE`
no hace nada. Las cuatro pantallas del tab bar son las únicas alcanzables.

Tiempo a contenido por pantalla, 5 repeticiones cada una, alternando la pantalla de origen:

| pantalla | n | mediana | rango | resolución del método |
|---|---:|---:|---|---|
| Discover | 5 | 2268 ms | 1930–2430 ms | 2–4 capturas |
| Backpack | 5 | 1851 ms | 1220–1955 ms | 2–4 capturas |
| Profile | 5 | 1160 ms | 1101–1182 ms | 2 capturas |
| Settings | 5 | 514 ms | 499–525 ms | **≤ 500 ms, no resoluble** |

Método: tap en el tab y `screencap` en bucle (~0.5 s por captura en este teléfono) hasta que el área
de la pantalla —todo menos la barra de tabs, que cambia en el propio tap— deja de diferir de una
referencia ya asentada de esa pantalla. El piso de resolución es una captura: Settings pinta antes de
la primera, así que se publica como cota superior, no como valor.

**Sin fuga de navegación**: 20 ciclos Discover↔Profile llevan el PSS de 2075 a 2021 MB (−55.9 MB,
monótono a la baja, `navleak.csv`). El piso de ruido es alto porque el mundo Godot sigue residente
detrás del shell (~2 GB): este test sólo vería una fuga grande, y el heap de JS no es legible en un
build release.

**Tap spam**: 20 taps al centro de la pantalla a 150 ms, mismo pid (21219), PSS +1 MB, ninguna línea
`FATAL`/`ANR`/error de RN en logcat, y las capturas de antes y después son indistinguibles.

Lo que no se pudo medir, con evidencia en el README del bloque: `uiautomator dump` muere por OOM en
Discover (9 intentos, 0 bytes; la misma orden en Settings devuelve 37 KB), así que esa pantalla va
sólo por píxeles; en 1.x no hay árbol de accesibilidad que consultar; y no se abrieron las subpantallas
de Settings.

## 7. Hallazgos

1. **La cola es lo único que separa a los dos builds, y 2.x la tiene peor.** Misma mediana y mismo
   p95, pero el 1 % más lento de 2.x es 10.6 % peor y su p99 es 11.9 % más alto, con la CPU 10.3 %
   más baja. No es falta de throughput: es hitching periódico. Los frames peores están en
   `frames.csv` de cada corrida (máx 100 ms en 2.x vs 92 ms en 1.x) y las trazas de Perfetto los
   ubican — candidato a investigar en una issue aparte: trabajo del hilo JS/RN intercalado con el
   render, o GC.
2. **El sol pesa más que el build.** Con `Dynamic Skybox` OFF y la misma configuración, 2.x pasó de
   31.3 fps (sol alto) a 13.5–15.2 fps (sol bajo): 2.2×. Se descubrió después de cerrar la primera
   versión y obligó a invalidar la matriz `HIGH` (§1.1).
3. **El panel de ajustes puede mentir.** Tres de seis corridas de `gp-hi-pair/` arrancaron en profile
   0 o 2 con el panel mostrando `High`; la única prueba válida del perfil es el `[RenderReady]` del
   pid de esa corrida, no lo que dice la UI ni un `logcat` sin filtrar.
4. **iOS quedó medible en fps y con ajustes scriptables.** El HUD del engine aparece en el árbol de
   accesibilidad (muestreo a 1 Hz, sin frame timeline) y `Documents/settings.json` se escribe desde
   `devicectl`, así que la matriz se puede correr en iPhone sin tocar el cliente.
5. **El fps medio no es comparable con dynamic graphics ON.** Los dos builds se autolimitan a
   ~18 fps en este teléfono. Para que el benchmark pueda ver diferencia haría falta un flag de
   arranque que lo desactive (y, del lado 1.x, que el runner `gp_benchmark` sea invocable también en
   2.x: hoy el shell RN usa `BOOT_ARGS` fijo y descarta los parámetros de engine del deeplink).
   Sin eso, cualquier comparación de fps en móvil mide el cap, no el cliente.
3. **2.x arranca y carga antes.** `[RenderReady]` t100 pasa de 7.7 s a 1.4 s y el arranque en frío
   de 27.8 s a 19.4 s. El `t100` alto de 1.x viene de compilar pipelines en serie; el port a Rust los
   compila en paralelo.
4. **2.x usa menos memoria, no más.** −3.2 % de PSS en mundo y −12.4 % en el test de 20 entradas
   (1917 → 1680 MB de mediana). Y ninguno de los dos fuga en 20 ciclos de entrar/salir.
5. **Falta un punto de entrada sin tocar la pantalla.** Ni el `gp-benchmark` de 1.x ni el flujo de
   invitado de 2.x se pueden disparar sin taps, y en iOS no hay inyección de input disponible en
   esta máquina: para medir iOS (y para CI) hace falta un arranque de benchmark por deeplink que
   entre al mundo con pose fija y sin flujo de login.
6. **El shell React Native no es donde se pierde el tiempo, y su salida se puede automatizar.**
   Cuatro pantallas, 0.5–2.3 s a contenido, sin fuga en 20 navegaciones y sin inestabilidad en 20
   taps en ráfaga. Para comparar contra 1.x habría que medir lo mismo por diff de capturas, porque
   en 1.x esas pantallas son del motor y no están en el árbol de accesibilidad.
7. **Cortar la red no da una plaza vacía**: los dos builds levantan un modal bloqueante de
   "Connection lost" (ver §9). Una medición con y sin avatares necesita el flag de comms, no red off.

## 8. Stress: soak de 71 minutos (2.x)

Una corrida continua de 71 min en Genesis Plaza, quieto, con el sampler tomando memoria, CPU y
temperatura cada ~3 s (1464 muestras). Datos en `data/…/android-a54/soak/2x-soak-60m/`.

| métrica | inicio | mediana | fin | pico |
|---|---:|---:|---:|---:|
| PSS | 2056 MB | 2049 MB | 2098 MB | 2206 MB |
| temperatura (máx / piel) | 48.9 / 38.0 °C | — | 47.5 / 40.0 °C | 48.9 / 40.2 °C |
| CPU | 139 % | 150 % | 160 % | — |

**Sin throttling térmico**: la temperatura no sube después del arranque (la piel queda en 38–40 °C
todo el run) y el CPU se mantiene plano.

**Memoria**: +42 MB entre la primera y la última muestra, con una deriva suave de ~+100 MB en la
mediana por tramos de 10 min (1996 → 2095 MB). No es un leak con escalón ni crece sin techo (el pico
de 2206 MB vuelve solo), pero tampoco es plano: en 71 min en la plaza el consumo sube lo suficiente
como para seguirle el rastro en un soak más largo.

**fps del soak: no disponible.** Con el buffer de Perfetto de 128 MB, 71 min de traza conservan sólo
los últimos ~6 min de frame timeline y pierden la tabla de procesos, así que la consulta de frames no
tiene a qué unirse (0 filas). El sampler sí cubrió los 71 min completos. La config ya quedó anotada
con la razón y el protocolo pide trocear el soak en ventanas de 10 min (protocolo §9.2) en vez de una sola
traza larga — eso es lo que hay que hacer la próxima vez; no se reintentó en esta pasada.

**UI por pantalla**: medida en §6 (sólo lado 2.x, porque en 1.x las pantallas las dibuja el motor
y no están en el árbol de accesibilidad). **Red degradada**: sin ejecutar (protocolo §9.5).

## 9. Intentos que no sirvieron (documentados para no repetirlos)

- **Plaza vacía cortando la red.** `NET_OFF_AFTER_READY=1` apaga Wi-Fi tras cargar el mundo para
  sacar los avatares remotos. Resultado: **los dos builds responden con el modal bloqueante
  "Connection lost / EXIT / RETRY"** (1.x además con blur de fondo), así que la ventana mide un
  diálogo sobre un mundo congelado. Datos y capturas en `data/…/gp-empty/` con `INVALID.md`.
  En 2.x no hay link que apague comms (`?pulse=false&livekit=false` no llega al motor porque el
  shell RN usa `BOOT_ARGS` fijo en `rn/android/app/src/godot/java/com/dclrn/MainActivity.kt`), y en
  1.x el runner `gp_benchmark` existe pero no se puede invocar por deeplink en 2.x → no hay forma
  común de tener plaza vacía con sesión viva.
- **fps como métrica principal.** Con dynamic graphics ON los dos builds se autolimitan a ~18 fps
  en este teléfono (`[DynamicGraphics] FPS limit changed, target=55.6ms`). Comparar fps medios es
  comparar el cap; por eso el resultado se apoya en p99/p1 y en la cola.

## 10. Limitaciones declaradas

1. **Dynamic graphics ON** en ambos (comportamiento de fábrica). No se forzó OFF desde Settings en
   ninguna de las dos apps: sin un flag de CLI equivalente (`graphic_profile`,
   `dynamic_graphics_enabled` sólo existen en Settings), hacerlo a mano en dos UIs distintas habría
   dejado las corridas sin condición verificable. Consecuencia: el fps medio no discrimina.
2. **Entornos distintos**: el build 2.x es de `main` y reporta `-staging` en el HUD; el 1.x de
   `release` reporta `-prod`. Misma escena (`scene_id` idéntico) pero no necesariamente los mismos
   servicios.
3. **N de avatares no controlado**: población real de la plaza (5–10 avatares), sin bot farm. El
   test de "muchos avatares hasta el punto de quiebre" queda pendiente.
4. **Un solo dispositivo por plataforma** (A54 y iPhone 16e); no hay resultados de otras GPUs.
5. **Instalación limpia en cada cambio de build**: Android rechaza instalar 1.16.1 encima de 2.0.0
   (`INSTALL_FAILED_VERSION_DOWNGRADE` incluso con `-d`), así que cada cambio borra datos. Los dos
   builds reciben el mismo trato y el mismo número de corridas, pero el estado del caché de
   contenido no es idéntico corrida a corrida (afecta a `cold_launch_to_render_ready`, no a la
   ventana de fps).
6. **iOS sin mundo**: ver §5.
7. **CPU**: `dumpsys cpuinfo` devuelve `+0%` en Android 16; la cifra sale de `top -b -n 1`. La de
   las corridas de humo iniciales quedó en 0 por ese bug; las de la tabla vienen del sampler
   corregido.
8. **Overlay de primer uso en 1.x**: ver §1. Las corridas de 1.x muestran el cartel de tutorial y
   los globos de movimiento/emotes; las de 2.x no. Es UI extra a favor de 2.x en la comparación.
9. **UI sólo con lado 2.x y con piso de 0.5 s**: ver §6. En 1.x esas pantallas se dibujan en el motor
   y no hay árbol de accesibilidad, así que no se midieron; y el probe por capturas no distingue nada
   por debajo de una captura (~0.5 s), que es el caso de Settings.

10. **La matriz `HIGH` definitiva se mide pareada.** El preset de cielo renderiza distinto en cada
    build (mismo label, luma 75 vs 105–112), así que se alternan builds con la misma configuración y
    se compara sólo entre corridas de luma cercana; el número final es un par, no un promedio suelto.
11. **`summarize.py` no corrige por luma.** Agrega por corrida y puede juntar en una misma mediana dos
    corridas con el sol en posiciones distintas; las tablas de `gp-hi-pair/` lo muestran.
12. **iOS no tiene frame timeline.** El fps sale de muestrear el HUD cada 1–2 s: hay fps medio, no hay
    p50/p99 de frame time en iOS.

## 11. Qué haría falta para cerrar lo que falta

- **Plaza vacía**: un flag de arranque en 2.x que llegue al motor (hoy el shell RN ignora los
  parámetros de engine), o el runner `gp_benchmark` invocable también en 2.x. Con eso, el mismo
  `run_scenario.sh gp` con `?gp-benchmark=true&pulse=false` da plaza vacía y pose pinneada en las
  dos ramas.
- **N avatares**: bot farm (un cliente de escritorio por bot con perfil propio, todos a `0,0`), o
  un lugar poblado con contador de avatares por captura.
- **UI por pantalla, con lado 1.x**: la mitad 2.x ya está medida (§6). Falta medir las mismas
  pantallas en 1.x, que se dibujan en el motor y no están en el árbol de accesibilidad: hace falta el
  mismo probe por diff de capturas que se usó del lado 2.x. Y del lado 2.x falta bajar el piso de
  resolución de 0.5 s (`screenrecord` con timestamps de frame en vez de `screencap`).
- **Soak con fps**: el soak de 71 min ya cubre memoria/térmica/CPU; falta trocearlo en ventanas de
  10 min para conservar la frame timeline (o subir el buffer de Perfetto).
- **Red degradada**: comandos listos (protocolo §9.5), sin ejecutar en esta pasada.

## 12. Datos crudos

```
docs/bench/data/2x-vs-1x-3206/
  android-a54/
    summary.md|json      tabla agregada que genera summarize.py (la del §1)
    gp/                  10 corridas (5 por build): meta.json, frames.csv, mem.csv,
                         thermal.csv, cpu.csv, screen.png, launch-*.log
    gp-empty/            6 corridas + INVALID.md (modal "Connection lost", fuera del reporte)
    rss-states/1x/       states.csv (40 muestras de meminfo), states-median.csv, capturas
    enter-exit/1x|2x/    cycles.csv (20 ciclos por build) + captura del ciclo 1 y del 20
    soak/2x-soak-60m/    mem.csv / thermal.csv / cpu.csv de 71 min + meta.json + screen.png
    ui-2x/               shell React Native: README con el método, ttc-pixel.csv / ttc-a11y.csv
                         (tiempo a contenido), navleak.csv, tapspam.csv, explore/ con capturas
  iphone16e/launch/      1x-mode/ y 2x/: console-*.log + launch-*.json
```

## 13. Reproducir

```bash
# 1) builds pinneados
curl -o .bench-dl/apk-1x-release-latest.apk \
  https://mobile-artifacts.dclregenesislabs.xyz/android/release/latest/decentraland.godot.client.apk
curl -o .bench-dl/apk-2x-e3b7aa102.apk \
  https://mobile-artifacts.dclregenesislabs.xyz/android/main/e3b7aa102bd0fbfe3a51cc2204248a59662b3fde/decentraland.godot.client.apk

# 2) matriz de Genesis Plaza, 5 corridas por build alternadas
ORDER="2x 1x 2x 1x 2x 1x 2x 1x 2x 1x" SETTLE=25 \
  scripts/bench/2x-vs-1x/run_scenario.sh gp "" 5 60

# 3) tabla agregada
python3 scripts/bench/2x-vs-1x/summarize.py --root docs/bench/data/2x-vs-1x-3206/android-a54 \
  --out docs/bench/data/2x-vs-1x-3206/android-a54/summary.md
```
