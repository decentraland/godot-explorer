# Roadmap de optimización a partir del benchmark (#3206)

Qué hacer con lo que salió medido en [`2x-vs-1x-3206-report.md`](./2x-vs-1x-3206-report.md).
Todo lo de acá son **hipótesis derivadas de los datos**, no hallazgos: nada está implementado ni
verificado con un cambio. Cada ítem dice con qué se comprueba y con qué se mide, porque en este
repo una optimización sin antes/después bajo el mismo protocolo es una opinión.

Orden: primero lo que **desbloquea medir** (sin eso las otras no se pueden evaluar), después las dos
cosas que la medición ya señala como caras, y al final lo que sólo tiene sentido con más datos.

## Lo que la medición fija como punto de partida

| hecho medido | número | de dónde |
|---|---|---|
| Con `dynamic graphics OFF` + perfil `HIGH` (arranque térmico emparejado), 2.x hace ~2× los fps de 1.x | 2.x 30.5–31.3 fps (p50 33.3 ms) · 1.x 13.5–24.5 fps (p50 58–75 ms) | `gp-hi-pair/` |
| Con los defaults que se entregan (dynamic `ON`), los dos quedan clavados | 17.9 vs 18.1 fps, p50 58.3 vs 58.2 ms | §1 del reporte |
| La cola de 2.x es peor donde el cap ya no manda | p99 +11.9 %, fps p1 −10.6 %, con CPU −10.3 % | §1 |
| 2.x ya gana en arranque y en memoria | `[RenderReady]` t100 7.7 s → 1.4 s; frío 27.8 s → 19.4 s; PSS −3.2 % / −12.4 % | §2, §4 |
| 71 min sin throttling; memoria con deriva suave | piel 38–40 °C; +42 MB entre primera y última muestra | §8 |
| 1.x varía 13.5↔24.5 fps con los mismos settings; 2.x repite al centésimo | p50 33.31 / 33.32 ms en dos corridas de 2.x | `gp-hi-pair/` |

Consecuencia de diseño: **el fps con dynamic graphics ON no sirve como métrica de progreso**. Donde
hay que mirar es (a) la cola de frame times con el cap apagado, (b) el costo por avatar/escena densa,
(c) el tiempo de arranque, y (d) la memoria en soak.

---

## P0 — Desbloquear la medición (sin esto, todo lo demás es fe)

### P0.1 · Que el deeplink de 2.x configure el motor

**Evidencia.** El shell RN arma `BOOT_ARGS` fijo en
`rn/android/app/src/godot/java/com/dclrn/MainActivity.kt`, así que los parámetros de engine que 1.x
acepta por deeplink (`?gp-benchmark=true&pulse=false`, quality, etc.) **se descartan en 2.x**. Por eso
la comparación publicada del §1 se hizo con dynamic graphics ON y quedó midiendo el cap, y por eso la
plaza vacía no se pudo medir (§9, §11).

**Acción.** Mapear los parámetros del deeplink a `BOOT_ARGS` (y a los `--quality`/`--profile` que ya
existen), al menos: perfil gráfico, dynamic graphics on/off, benchmark runner, pose fija y
`pulse=false`. Nada nuevo de engine: es plumbing de intent → argumentos.

**Validación.** Dos corridas del mismo APK, una con dynamic ON y otra OFF, deben dar números
distintos y `[DynamicGraphics] initialized: enabled=false, profile=3` en el `launch.log`.

**Costo.** S. **Riesgo.** Bajo (parámetros de arranque, no lógica de juego).

### P0.2 · Un contador de frames y de carga de escena dentro del cliente

**Evidencia.** Medir fps hoy depende de Perfetto en Android y **en iOS no hay fuente**: el §5 sólo
pudo medir arranque y footprint. Además el soak de 71 min perdió la frame timeline porque el buffer
de Perfetto no daba para tanto (§8).

**Acción.** Exponer en el debug hub (y por log a intervalo fijo) lo mínimo: fps, p50/p95/p99 de
`delta` de los últimos N frames, frames > 50 ms, y del lado del renderer `draw_calls`,
`objects_in_frame`, `primitives_in_frame`, `texture_mem`. Godot ya tiene esos contadores; falta
publicarlos.

**Validación.** El número tiene que coincidir con el que da Perfetto en una ventana de 30 s (si no
coincide, el instrumento nuevo no sirve).

**Costo.** S/M. **Riesgo.** Bajo. Es lo que habilita P1.1, P2.1 y todo iOS.

### P0.3 · Un gate de performance en CI

**Evidencia.** Este benchmark necesitó ~10 h de teléfono, cooldown y taps manuales para una sola
comparación. Cualquier optimización futura se va a validar igual de caro, así que no se va a validar.

**Acción.** Con P0.1 + P0.2: un job (device farm o un A54 colgado de un runner) que corra `gp`,
`gp` con HIGH y un soak de 20 min, y compare contra una baseline versionada. Falla por umbral
(p99, fps, tiempo a `RenderReady`, PSS a los 20 min), no por comparación entre builds distintos.

**Costo.** M. **Riesgo.** Medio: un gate que flapea se ignora y ensucia el CI. Empezar por `warn` y
promover a `fail` cuando la dispersión medida (< 1 % en 2.x) lo permita.

---

## P1 — Lo que la medición ya señala

### P1.1 · Cazar los hitches de 2.x (p99 +11.9 %, máx 100 ms)

**Evidencia.** Con el cap puesto, 2.x gasta **menos** CPU que 1.x y aun así tiene el 1 % de frames
peores 10.6 % peor. No es throughput: es trabajo periódico que cae dentro del frame.

**Hipótesis, en orden de probabilidad.** (1) GC del runtime JS/deno con pausas que caen en el frame;
(2) trabajo del hilo del shell RN intercalado con el render (bridge, analytics, imágenes);
(3) subida de texturas/mallas en el frame crítico al entrar avatares nuevos; (4) transición de
LOD/impostor de avatares; (5) flush de CRDT de escena en un frame grande.

**Acción.** Correr 60 s con marcadores de fase por frame (tick JS, flush CRDT, física, submit de
render) y cruzar con `frames.csv`: buscar qué fase correlaciona con los frames > 50 ms. Con
P0.2 sale del propio cliente; sin P0.2, con Perfetto + `RUST_LOG` de los sistemas involucrados.

**Validación.** El mismo protocolo de `gp-hi-pair` antes/después: p99 y `fps p1` bajan, mediana y
PSS no cambian.

**Costo.** M. **Riesgo.** Bajo: es diagnóstico, no cambio de comportamiento.

### P1.2 · Costo por avatar y por densidad de escena

**Evidencia.** Con HIGH, 1.x pasó de 24.5 fps en una corrida con 2–3 avatares cerca a 13.5 fps con
~8 (pantallas guardadas en cada corrida), mientras 2.x se quedó en 30.5–31.3 con 5–8 avatares.
También la 1.x del §1 se movía 5–10 avatares. **Es una observación visual, no una curva**: 1.x no
expone contadores y no se fijó N.

**Acción.** Con P0.1 (o un bot farm de clientes de escritorio, §11 del reporte) medir fps vs N
avatares en 0/1/5/10/20, para los dos builds. Si el costo de 2.x crece de forma lineal con N y el de
1.x cuadrática, la diferencia de P1 ya está explicada; si 2.x también se dobla, el LOD/impostor de
avatares es la próxima palanca real (umbral de distancia, calidad del impostor, `AvatarScene`
multi-mesh ya existe para eso).

**Costo.** M (medición) + S/M (ajustes). **Riesgo.** Medio: tocar LOD degrada visualmente; hay que
medirlo con capturas, no sólo con fps.

### P1.3 · Política de dynamic graphics

**Evidencia.** El controlador arranca con `enabled=false, profile=3, target=16.6 ms` cuando se apaga
a mano; con dynamic ON los dos builds quedan en ~18 fps con target 16.6 ms, o sea que el objetivo
declarado y el resultado no coinciden y el perfil oscila sin que se vea la decisión. Con HIGH y
cap apagado el mismo teléfono hace 31 fps: hay margen entre lo que el controlador cree y lo que el
dispositivo puede.

**Acción.** Revisar el target por tier de dispositivo (16.6 ms es un objetivo de 60 fps que el A54
no alcanza a HIGH), agregar histéresis para no oscilar entre perfiles, y loguear cada decisión
(`profile X → Y, motivo`). Medible sin tocar render.

**Validación.** Con dynamic ON: dispersión de p50 dentro del run y cantidad de cambios de perfil por
minuto, antes/después.

**Costo.** S. **Riesgo.** Bajo, pero cambia lo que ve el usuario: esto **no** es un cambio que
corresponda a este issue, va a issue propia.

---

## P2 — Después de P0 y P1, y sólo con datos del renderer

### P2.1 · ¿GPU o CPU? Partir el frame

**Evidencia.** En `gp-hi-pair` 2.x sostiene 30 fps con CPU ~185 % (p1) y 2.x con el cap puesto usa
119 %; el soak mantiene 139–160 %. Con esos números no se sabe si el techo es submit de CPU o
relleno de GPU, y elegir mal la palanca quema una semana.

**Acción.** Con P0.2 mirar `primitives_in_frame`, `draw_calls` y el tiempo de GPU por frame en GP.
Si es GPU: bajar fill (resolución interna, AA, sombras, bloom) por perfil — es lo que el perfil HIGH
ya toca. Si es CPU: draw calls e instancing de props repetidos de la plaza (carteles, guirnaldas,
banners) y el costo de los sistemas por componente del scene runner.

**Costo.** M. **Riesgo.** Bajo de medir, alto de tocar (calidad visual).

### P2.2 · Arranque: 19.4 s en frío todavía es mucho

**Evidencia.** 2.x ya bajó `[RenderReady]` t100 de 7.7 s a 1.4 s y el frío de 27.8 s a 19.4 s, pero
19 s hasta ver mundo sigue siendo la mayor queja de uso posible. Y `am start -W TotalTime` **empeoró**
(330 → 481 ms): el proceso tarda más en arrancar aunque el mundo aparezca antes.

**Acción.** Instrumentar por fase con los marcadores que ya existen (`RenderReady` t90/t100) más
tiempo de descarga de contenido y de compilación de pipelines, y atacar la fase más grande. El shell
RN aporta su propio arranque (ver P3.1).

**Costo.** M. **Riesgo.** Bajo.

---

## P3 — Deuda que sólo duele si se ignora

### P3.1 · Tiempo a contenido del shell RN

**Evidencia.** §6: Discover 2.27 s, Backpack 1.85 s, Profile 1.16 s, Settings ≤ 0.5 s. Sin fugas
(−56 MB en 20 navegaciones) y sin crashes.

**Acción.** Donde el número es alto, casi siempre es imagen remota: precargar el primer frame de las
cards y del avatar, cache en disco con placeholder, y render de la lista antes de tener imágenes.
No tocar Settings, que ya está bien.

**Validación.** Mismo probe de capturas (§6) o `screenrecord` con timestamps para bajar el piso de
0.5 s.

**Costo.** S/M. **Riesgo.** Bajo.

### P3.2 · Deriva de memoria en soak

**Evidencia.** §8: +42 MB entre la primera y la última muestra de 71 min, con una deriva suave de
~+100 MB en la mediana y sin escalones. No es una fuga demostrada (no hay tendencia sostenida) pero
tampoco está explicada; el PSS pico fue 2206 MB.

**Acción.** Soak de 20 min como test (no de 71), con corte por pendiente; y si la pendiente aparece,
atribuir con `dumpsys meminfo` por categoría (Java/native/graphics) y con el cache de contenido, que
es el sospechoso obvio.

**Costo.** S. **Riesgo.** Bajo.

### P3.3 · iOS medible

**Evidencia.** §5: iOS sólo tiene arranque y footprint porque no había fuente de fps ni inyección de
input. La inyección ya está resuelta (`~/bin/iphone`, Appium + WebDriverAgent, sesión verificada con
captura); la fuente de fps depende de P0.2.

**Acción.** Enchufar el probe de P0.2 al log del cliente en iOS y correr el mismo escenario con el
harness de Appium. Es lo que convierte "2.x en iOS no sabemos" en un número.

**Costo.** S una vez que exista P0.2. **Riesgo.** Bajo.

### P3.4 · Que una corrida inválida se invalide sola

**Evidencia.** En esta campaña se publicaron y luego se tiraron tres corridas por estado oculto: la
1.x midiendo defaults por un doble tap, la 1.x con el panel abierto (15 fps en vez de 24), y dos
veces el mundo reanudado tras reinstalar. Costó horas y una de esas corridas llegó a estar commiteada.

**Acción.** Que el runner escriba y verifique un "fingerprint de estado" por corrida: perfil aplicado
(log en 2.x, captura en 1.x), panel cerrado, cantidad de avatares en pantalla, `[RenderReady]`
presente, temperatura de arranque. Si el fingerprint no matchea, la corrida se marca `INVALID.md`
sola y `summarize.py` la saltea (ya lo sabe hacer).

**Costo.** S. **Riesgo.** Bajo. Es lo que evita volver a discutir si un número era real.

---

## Lo que no priorizaría todavía

- **Micro-optimizar el frame medio.** Con dynamic ON empatan por diseño y con el cap apagado 2.x ya
  duplica a 1.x. No hay problema demostrado en la mediana.
- **Memoria.** 2.x usa menos en todos los estados medidos, sin fuga en 20 ciclos. Sin evidencia de
  problema, optimizarla es trabajo sin métrica.
- **Tocar la calidad visual por defecto** para ganar fps. El usuario de móvil ya elige perfil, y el
  benchmark muestra que el problema no es el techo del render sino la política que decide el techo
  (P1.3) y la cola (P1.1).
- **Comparar fps entre builds con la configuración que se entrega hoy.** Mide el cap. Primero P0.1.

## Orden sugerido

1. **P0.1 + P0.2** (S+S, misma semana): sin esto no hay forma barata de validar nada.
2. **P1.1 + P1.2** (M+M): los dos candidatos que la medición ya señala con número.
3. **P1.3 + P3.4 + P3.2** (S+S+S): baratos, y P3.4 protege la credibilidad de todo lo anterior.
4. **P0.3** cuando la dispersión medida lo permita (2.x repite al centésimo, pero 1.x no).
5. **P2.x y P3.1/P3.3** con los datos del renderer ya en mano.
