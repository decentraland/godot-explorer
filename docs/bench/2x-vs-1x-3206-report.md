# 2.x vs 1.x — benchmark medido (#3206)

Medición del cliente 2.x contra el 1.x en el mismo teléfono, misma escena y mismo pose, con el
protocolo de [`2x-vs-1x-3206-protocol.md`](./2x-vs-1x-3206-protocol.md) (comandos exactos y criterios
de invalidación). Datos crudos en [`data/2x-vs-1x-3206/`](./data/2x-vs-1x-3206/).

## TL;DR

- **fps**: empatados por diseño. Ambos builds corren con dynamic graphics ON y quedan clavados en
  ~18 fps (objetivo 55.6 ms/frame) en Genesis Plaza. 1.x **17.9**, 2.x **18.1** (mediana de 5
  corridas de 60 s cada una, alternadas). El cap esconde la diferencia: el número no distingue los
  builds, lo que distingue es la **cola**.
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
  pequeño), ver §7.
- **Presupuesto** del issue: fps ≥ 1.x → **PASS**; RSS ≤ 1.x + 100 MB → **PASS** (2.x usa menos).

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
forma de fijar N (§9 de limitaciones), así que este bloque es "plaza poblada tal cual", no un test de
N avatares.

**Diferencia de overlay a declarar.** En las corridas de 1.x quedó arriba el cartel de primer uso
"Tap the screen to start exploring" con los globos "Character movement"/"Emotes"
(`1x-gp-r4/screen.png`), mientras que en 2.x el tutorial se había descartado en la preparación
(`SKIP`). Son overlays del motor dibujados sobre el mundo: suman UI a 1.x durante la ventana. No
invierte el resultado (2.x es peor en la cola, no mejor), pero es una asimetría real y se declara.

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
un estado de memoria de menús. En 2.x el control de salida está dentro del shell React Native y no se
encontró de forma scriptable en esta pasada — ver §10.

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

## 6. Hallazgos

1. **La cola es lo único que separa a los dos builds, y 2.x la tiene peor.** Misma mediana y mismo
   p95, pero el 1 % más lento de 2.x es 10.6 % peor y su p99 es 11.9 % más alto, con la CPU 10.3 %
   más baja. No es falta de throughput: es hitching periódico. Los frames peores están en
   `frames.csv` de cada corrida (máx 100 ms en 2.x vs 92 ms en 1.x) y las trazas de Perfetto los
   ubican — candidato a investigar en una issue aparte: trabajo del hilo JS/RN intercalado con el
   render, o GC.
2. **El fps medio no es comparable con dynamic graphics ON.** Los dos builds se autolimitan a
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
6. **Cortar la red no da una plaza vacía**: los dos builds levantan un modal bloqueante de
   "Connection lost" (ver §8). Una medición con y sin avatares necesita el flag de comms, no red off.

## 7. Stress: soak de 71 minutos (2.x)

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

**UI por pantalla y red degradada**: sin ejecutar (§10).

## 8. Intentos que no sirvieron (documentados para no repetirlos)

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

## 9. Limitaciones declaradas

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

## 10. Qué haría falta para cerrar lo que falta

- **Plaza vacía**: un flag de arranque en 2.x que llegue al motor (hoy el shell RN ignora los
  parámetros de engine), o el runner `gp_benchmark` invocable también en 2.x. Con eso, el mismo
  `run_scenario.sh gp` con `?gp-benchmark=true&pulse=false` da plaza vacía y pose pinneada en las
  dos ramas.
- **N avatares**: bot farm (un cliente de escritorio por bot con perfil propio, todos a `0,0`), o
  un lugar poblado con contador de avatares por captura.
- **UI por pantalla**: mapear las pantallas del shell RN 2.x (hoy se llega al mundo y no se vuelve
  al shell de forma scriptable) y medir tiempo a interactiva por pantalla con `ui.sh` (polling de
  `uiautomator`), que es lo que sí expone 2.x; en 1.x la UI es dibujada por el motor y no aparece en
  el árbol de accesibilidad, así que ese bloque sólo puede hacerse con diff de capturas.
- **Soak con fps**: el soak de 71 min ya cubre memoria/térmica/CPU; falta trocearlo en ventanas de
  10 min para conservar la frame timeline (o subir el buffer de Perfetto).
- **Tap spam y red degradada**: comandos listos (protocolo §9.4–9.5), sin ejecutar en esta pasada.

## 11. Datos crudos

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
    ui-2x/               shell React Native: qué se pudo y qué no medir, con capturas
  iphone16e/launch/      1x-mode/ y 2x/: console-*.log + launch-*.json
```

## 12. Reproducir

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
