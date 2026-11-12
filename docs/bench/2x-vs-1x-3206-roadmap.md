# Roadmap de optimización a partir del benchmark (#3206)

Qué hacer con lo que salió medido en [`2x-vs-1x-3206-report.md`](./2x-vs-1x-3206-report.md).
Todo lo de acá son **hipótesis derivadas de los datos**: nada está implementado ni verificado con un
cambio. Cada ítem dice qué evidencia lo sostiene, qué se haría, cómo se comprueba y contra qué número
se acepta. En este repo una optimización sin antes/después bajo el mismo protocolo es una opinión.

Los ítems tienen ID (`A1`, `C2`…) para poder linkearlos desde issues, y `depende de` cuando el orden
importa.

---

## 0. Resumen ejecutivo

1. **Con la condición controlada, los builds empatan y la cola sigue siendo de 2.x.** Mundo fijo
   (`monsterrecon.dcl.eth`), `Midday` fijo, perfil `High` verificado por su efecto, misma térmica:
   **1.x 40.22 fps vs 2.x 38.62 fps (−4.0 %)**, IC95 [−4.18, +1.83], MWU exacto p = 0.24 → **sin
   diferencia demostrada** con 6 ventanas por brazo (§1.3 del reporte). Lo que sí separa: **p99 +11.9 %**,
   **fps p1 −10.6 %**, con 10.3 % menos CPU. El trabajo periódico dentro del frame (J1/B1) sigue siendo
   el primer workstream; el throughput no.
   *Retractado*: un bloque anterior publicó −50.9 % en esta misma condición. Era el perfil `Very Low`
   (cap 18 fps) sin aplicar por el shell 2.x, no coste de render; se detectó midiendo el escalón
   Medium (29.96 = su cap de 30) contra High (38.62) en la misma sesión.
2. **La medición estaba dominada por una variable que nadie estaba fijando: el sol.** Mismo build y misma
   temperatura de arranque: 31.3 fps con el sol alto contra 14.4 con el sol bajo, **2.2×**, más que la
   diferencia entre builds. La iluminación pasó a ser el primer workstream técnico (C), y el sol dejó de
   tratarse como un ajuste (punto siguiente, ítem A1).
3. **El sol no se puede fijar desde el cliente.** Con `Dynamic Skybox` OFF y `Custom Skybox = Midday`
   en las dos builds, el mismo cliente siguió midiendo ventanas de luminancia media 53 y 140 en la misma
   sesión: en Genesis Plaza el reloj lo maneja la escena (`SkyboxTime`), no el ajuste. Además el mismo
   preset renderiza distinto en cada build (luma ~75 en 2.x, ~105–112 en 1.x). Consecuencia: el sol pasa
   a tratarse como **covariable medida**, no como variable controlada, y la comparación se hace
   muestreando cada build a lo largo del ciclo y comparando a **luminancia pareja** (`run_drift.sh`).
4. **Antes de tocar nada hay que poder medir barato.** Hoy medir una cosa cuesta ~10 h de teléfono,
   taps a ciegas y cooldown manual; el instrumento (Perfetto) no existe en iOS y no aguanta un soak
   largo. Los ítems del workstream A no optimizan nada: hacen que los demás sean evaluables.

Lo que **no** priorizaría todavía: frame medio, memoria y bajar calidad por defecto (ver §4).

---

## 1. Punto de partida medido

Device: Samsung Galaxy A54 5G (`SM-A546E`), Android 16, arm64-v8a, 120 Hz. Escena: Genesis Plaza,
cámara quieta en el spawn, ventana de 30 s con el teléfono enfriado a ≤34 °C antes de cada corrida.

| Hecho medido | Número | Dónde |
|---|---|---|
| Con `dynamic OFF` + perfil `HIGH` no se puede comparar tal como se midió | 1.x luma 86–118 vs 2.x 57–78; el fps sigue a la luz | `gp-hi-pair/` (inválido), `gp-hi-v3/`, `gp-sun-drift/` |
| El sol manda sobre todo lo demás | 2.x: 31.3 fps (luma 117) → 14.4 fps (luma 78), mismo build, mismo perfil, 2.2× | `gp-hi-pair/2x-hi-p1` vs `gp-hi-fixed-sky/2x-sky-r2` |
| El perfil hay que verificarlo en el log, no en el panel | 3 de 6 corridas de `gp-hi-pair` arrancaron en profile 0 o 2 con el panel mostrando `High` | `gp-hi-pair/INVALID.md` |
| El preset de cielo no significa lo mismo en los dos builds | `Morning`: luma 75 (2.x) vs 105–112 (1.x); `Midday` en 1.x mueve la luz a luma 140 | `gp-hi-fixed-sky/`, `gp-hi-midday/` |
| iOS sí tiene fuente de fps | El HUD del engine expone `"…- 60 FPS"` en el árbol de accesibilidad; muestreo a 1 Hz | `iphone ui` (iPhone 16e) |
| Con los defaults que se entregan (dynamic `ON`) los dos empatan | 17.9 vs 18.1 fps · p50 58.3 vs 58.2 ms | reporte §1 |
| La cola de 2.x es peor | p99 +11.9 % · fps p1 −10.6 % · máximo 100 ms vs 92 ms · CPU −10.3 % | reporte §1 |
| 2.x gana en arranque y memoria | `RenderReady` t100 7.7 s → 1.4 s · frío 27.8 s → 19.4 s · PSS −3.2 % / −12.4 % | §2, §4 |
| 71 min sin throttling | piel 38–40 °C, CPU plano, +42 MB entre primera y última muestra | §8 |
| Repetibilidad asimétrica | 2.x p50 33.31 / 33.32 ms · 1.x saltó 13.5–24.5 fps | `gp-hi-pair/` |
| El shell RN no es el cuello | TTC 0.5–2.3 s por pantalla, sin fuga en 20 navegaciones | §6 |

**Lectura**: hay dos poblaciones de números en este benchmark, y la diferencia entre ellas es más
grande que la diferencia entre clientes. Cualquier trabajo de performance tiene que empezar por
reducir esa varianza (workstream A), o cada medición va a discutir el instrumento en vez del código.

---

## 2. Workstreams

Cada ítem: **E**videncia · **A**cción · **V**alidación · costo · riesgo.

### A. Instrumentación, condiciones y automatización

Es el workstream que no optimiza pero habilita todo lo demás.

**A1 · Que el ajuste de cielo llegue al motor (bloquea toda la matriz HIGH).**
- **E.** En 2.x, `Dynamic Skybox` OFF + `Custom Skybox = Midday` guardado (el panel lo muestra y el
  preview pinta día) renderiza **noche**: luma 33.1 en la parcela 100,100, contra 110.6–111.9 del mismo
  ajuste en 1.x. Causa: `sky/skybox_time.rs` lee `setting("skybox_time").try_to::<f64>().unwrap_or(0.0)`
  y `0.0` es medianoche, así que un valor que no llega como número cae en la hora más oscura sin aviso.
- **A.** Verificar el valor en el borde: leerlo como `Variant` y aceptar entero, float o texto numérico;
  si no es convertible, dejar el reloj dinámico y emitir un warning en vez de asumir medianoche.
- **V.** Con el ajuste en cada uno de los cuatro presets, la luma del mundo difiere en ≥ 40 puntos entre
  Midnight y Midday, y el valor leído coincide con el preset elegido en el log del engine.
- **Costo** S · **Riesgo** bajo. **Estado:** bug abierto, no tocado (este repo mide, no arregla).
- **Por qué primero:** sin esto no hay comparación posible con el sol fijo, y además es un bug visible:
  cualquiera que apague el skybox dinámico ve noche a cualquier hora.

**A2 · Fijar las condiciones de la escena (sol, N avatares, clima de calidad).**
- **E.** El sol no se puede fijar desde el cliente: con `Dynamic Skybox` OFF y `Custom Skybox = Midday`
  en las dos builds, la misma sesión de 2.x midió ventanas de luma 53.5 y 140.4, y el ciclo se mueve en
  minutos (muestreos consecutivos: 58.6 → 41.8 → 41.5 → 58.1). En Genesis Plaza el reloj lo controla la
  escena (`SkyboxTime`), y el mismo preset renderiza distinto en cada build (luma ~75 en 2.x,
  ~105–112 en 1.x).
- **A.** Dejar de intentar fijarlo: `run_drift.sh` muestrea **una** build N veces a lo largo del ciclo y
  guarda `sampling.csv` (luma, fps, p50, p99, cpu, temperatura, hora). La comparación entre builds se
  hace **a luminancia pareja**, no por medianas de poblaciones distintas.
- **V.** Dos builds con muestras en la misma banda de luma (|Δ| ≤ 5) y fps dentro de ±5 % para el mismo
  estado de luz.
- **Costo** S · **Riesgo** bajo. **Estado:** el runner ya guarda `state.json` con la luma y
  `run_drift.sh` muestrea la deriva; **bloqueado para 2.x** por A1;
  `gp-hi-fixed-sky/` y `gp-hi-midday/` quedan como evidencia de que el preset solo no alcanza.

**A3 · Args de engine por deeplink en 2.x.**
- **E.** El shell RN arma `BOOT_ARGS` fijo (`rn/android/app/src/godot/java/com/dclrn/MainActivity.kt`),
  así que los parámetros que 1.x acepta por deeplink (benchmark runner, `pulse=false`, calidad, pose)
  se descartan en 2.x. Por eso el §1 del reporte terminó midiendo el cap de dynamic graphics.
- **A.** Mapear el deeplink a los argumentos que el engine ya entiende, incluida pose fija y plaza
  vacía.
- **V.** El mismo APK con `?gp-benchmark=true&pulse=false` entra sin flujo de login y con pose fija;
  calidad y perfil quedan reflejados en el log de arranque.
- **Costo** S · **Riesgo** bajo (plumbing). **Depende de** nada.

**A4 · Contadores dentro del cliente (fps + renderer).**
- **E.** En Android se mide con Perfetto; **en iOS no hay fuente** (§5 sólo midió arranque y
  footprint) y el soak de 71 min perdió la frame timeline por tamaño de buffer.
- **A.** Log periódico y campos en el debug hub: fps, p50/p95/p99 del `delta`, frames > 50 ms,
  `draw_calls`, `objects_in_frame`, `primitives_in_frame`, `texture_mem`, y del lado escena:
  avatares visibles, entidades activas, tiempo de tick JS.
- **V.** Tiene que coincidir con Perfetto en la misma ventana de 30 s (si no, el instrumento no sirve).
- **Costo** S/M · **Riesgo** bajo. **Habilita** B1, C1, D1, K2.

**A5 · Gate de performance en CI.**
- **E.** Una sola comparación consumió ~10 h de teléfono, cooldown y taps.
- **A.** Job con un A54 colgado de un runner (o device farm) que corra tres escenarios (`gp` noon,
  `gp` dusk, soak de 20 min) contra una baseline versionada; falla por umbral, no por comparación
  entre builds.
- **V.** El gate detecta una regresión inyectada a propósito (p. ej. +20 % en p99).
- **Costo** M · **Riesgo** medio (un gate que flapea se ignora; empezar en `warn`). **Depende de** A1–A5.

**A6 · Fingerprint de estado por corrida y auto-invalidación.**
- **E.** Se tiraron cuatro corridas por estado oculto: 1.x midiendo defaults por un doble tap, 1.x con
  el panel abierto (15 vs 24 fps), dos veces el mundo reanudado tras reinstalar y una con el perfil
  puesto pero el `logcat` de otro lanzamiento.
- **A.** `state.json` por corrida (perfil cargado por el engine, cambios de perfil dentro de la
  ventana, luma, temperatura de arranque, avatares a la vista) + `INVALID.md` automático; el summary
  ya saltea escenarios inválidos.
- **V.** Una corrida hecha a propósito con el panel abierto tiene que quedar marcada sola.
- **Costo** S · **Riesgo** bajo. **Estado:** parcial (perfil + cambios + luma ya están).

### B. Frame pacing y la cola

**B1 · Cazar los hitches de 2.x.**
- **E.** p99 +11.9 %, fps p1 −10.6 %, máximos de 100 ms contra 92 ms, con 10.3 % menos CPU. Misma
  mediana y mismo p95: es periódico, no un techo.
- **A.** Marcadores de fase por frame (tick JS/deno, flush CRDT, física, upload de recursos, submit de
  render, trabajo del hilo RN) y cruce con `frames.csv`: qué fase explica los frames > 50 ms.
- **Hipótesis en orden de probabilidad:** GC del runtime JS · trabajo del bridge/hilo RN (analytics,
  imágenes, storage) · subida de texturas al entrar avatares nuevos · transición de LOD/impostor ·
  flush de CRDT de escena en un frame grande.
- **V.** p99 y fps p1 bajan con mediana y PSS intactos, en `gp` noon y en `gp` dusk.
- **Costo** M · **Riesgo** bajo (diagnóstico). **Depende de** A4.

**B2 · Presupuesto de frame explícito.**
- **E.** Hoy no hay un número que diga cuánto puede costar cada sistema; el fps resultante es la
  consecuencia. El HUD ya muestra fps pero nadie los compara contra un objetivo por fase.
- **A.** Definir presupuesto por fase para el tier del dispositivo (p. ej. a 30 fps: 12 ms render,
  6 ms escena/JS, 4 ms resto) y loguear la desviación por fase en builds de benchmark.
- **V.** Los frames que exceden el presupuesto total se pueden atribuir a una fase concreta.
- **Costo** S/M · **Riesgo** bajo. **Depende de** B1.

### C. Iluminación y ciclo día-noche

**C1 · Entender por qué el sol bajo cuesta 2.2×.**
- **E.** Mismo build, mismo perfil, misma temperatura: 31.3 fps con luma 117 vs 14.4 fps con luma 78
  (en 1.x el mismo patrón: 24.5 fps con luma 129 vs 15.5 con luma 78). La diferencia entre builds es
  de decenas de %, la del sol es de más del doble. El ciclo lo maneja la escena: con `Midday` fijado y
  el dynamic skybox apagado, la misma sesión midió luma 53.5 y 140.4, y el ciclo se mueve en minutos.
- **A.** Con A4: comparar a mediodía y de noche `draw_calls`, `objects_in_frame`, luces activas,
  overdraw y tiempo de GPU. Hipótesis: muchas luces/emisiones encendidas + IBL distinta + más
  materiales con blend.
- **V.** Se identifica la métrica que se dobla de día a noche; después, un cambio que acerque el
  costo nocturno al diurno sin que la captura cambie de forma evidente.
- **Costo** M · **Riesgo** medio (tocar iluminación cambia la cara del mundo).

**C2 · Presupuesto de luces y emisión por perfil.**
- **E.** Consecuencia de C1: si las luces son la causa, el perfil `HIGH` en móvil está pagando luces
  de escena que a 6" no se distinguen.
- **A.** Límite de luces dinámicas por perfil, emisión por material (barata) en vez de luces reales, y
  un pase de "luces cercanas al jugador" en vez de todas.
- **V.** Noche acotada a ≤10 % de costo sobre día, con capturas lado a lado aprobadas.
- **Costo** M · **Riesgo** alto (visual). **Depende de** C1.

### D. Avatares

**D1 · Curva fps vs N avatares.**
- **E.** Con `HIGH`, 1.x pasó de 24.5 fps (2–3 avatares cerca) a 13.5 fps (~8) con la misma
  configuración; en 2.x la muestra es chica (5–8 avatares, 30.5–31.3 fps). **No hay curva**: 1.x no
  expone contadores y no se fijó N.
- **A.** Medir 0/1/5/10/20 avatares en los dos builds (necesita A2 o un bot farm de clientes de
  escritorio con perfiles distintos).
- **V.** Dos curvas con pendiente y punto de saturación; sirven de línea base para D2/D3.
- **Costo** M · **Riesgo** bajo (medición). **Depende de** A2.

**D2 · Umbrales de impostor y LOD por avatar.**
- **E.** El sistema existe (`godot/src/decentraland_components/avatar/impostor/`, `AvatarScene` con
  `MultiMeshInstance3D` + `Texture2DArray`) pero no hay número que diga a qué distancia deja de
  convenir el avatar completo en móvil.
- **A.** Ajustar umbral por distancia y por cantidad de avatares en pantalla, con el costo medido en
  D1 en vez de a ojo.
- **V.** Con 10 avatares en pantalla, +20 % fps sin cambio perceptible en captura a 1 m de distancia
  de observación.
- **Costo** S/M · **Riesgo** medio. **Depende de** D1.

**D3 · Costo por avatar: animación, texturas y decode.**
- **E.** Sin datos todavía: es lo que D1 tiene que decir. Memoria de wearables y actualización de
  esqueletos son los sospechosos habituales en un cliente de este tipo.
- **A.** Medir por avatar el costo de skinning/animación y el peso de texturas; evaluar atlas por
  avatar y decode fuera del hilo principal.
- **V.** Costo por avatar acotado y lineal con N.
- **Costo** M · **Riesgo** medio. **Depende de** D1.

### E. Render

**E1 · ¿GPU o CPU? Partir el frame.**
- **E.** 2.x sostiene 30 fps con CPU ~185 % (a HIGH, cap apagado) y 119 % con el cap puesto. Con esos
  números no se puede elegir la palanca sin dividir el frame.
- **A.** Con A4, medir tiempo de GPU por frame y comparar con el tiempo de CPU; separar submit,
  shadows, post (bloom/AA) y geometría.
- **V.** Un veredicto con número: "en `gp` noon HIGH, el frame es X % GPU / Y % CPU en el p99".
- **Costo** M · **Riesgo** bajo. **Depende de** A4.

**E2 · Draw calls, instancing y batching de props repetidos.**
- **E.** Genesis Plaza repite carteles, guirnaldas, macetas y banners; hoy nadie sabe cuántos draw
  calls cuesta.
- **A.** Medir `draw_calls`/`primitives_in_frame` por zona; aplicar instancing/multimesh a los props
  repetidos y atlas de materiales donde aplique.
- **V.** −20 % draw calls en GP con captura idéntica y p99 mejor.
- **Costo** M/L · **Riesgo** medio. **Depende de** E1 (si es GPU-bound, esto no mueve la aguja).

**E3 · Presupuesto de sombras, AA y post por perfil móvil.**
- **E.** El perfil `HIGH` en móvil usa shadow quality 1 y bloom 2 por defecto; el costo relativo de
  cada uno no está medido en este device.
- **A.** Ablation: una corrida por componente apagado (sombras, bloom, AA) para tener el precio de
  cada uno a 30 fps.
- **V.** Tabla de precio por componente; el perfil móvil se reordena según eso.
- **Costo** S · **Riesgo** bajo (medición), alto al aplicar.

### F. Runtime de escena y JS

**F1 · GC y asignaciones por frame.**
- **E.** La cola de 2.x con menos CPU apunta a pausas, y el runtime JS es el candidato más probable.
- **A.** Instrumentar el heap del runtime, contar GCs y su duración, y cazar asignaciones por frame en
  los sistemas calientes (`scene_runner`, sistemas por componente).
- **V.** Cero GCs dentro de un frame de 33 ms en `gp`; p99 sin los picos de 100 ms.
- **Costo** M · **Riesgo** medio. **Depende de** B1.

**F2 · Costo por componente del scene runner.**
- **E.** No hay desglose por sistema; el tick de escena es una caja.
- **A.** Cronometrar por sistema en builds de benchmark y publicar el top 10 por escena (GP como
  referencia).
- **V.** Ranking estable entre corridas (±5 %).
- **Costo** S/M · **Riesgo** bajo. **Depende de** A4.

### G. Arranque y contenido

**G1 · Arranque en frío de 19.4 s.**
- **E.** Ya bajó de 27.8 s, pero 19 s hasta ver mundo sigue siendo el peor momento de uso, y
  `am start -W TotalTime` **empeoró** (330 → 481 ms): el proceso tarda más en arrancar aunque el
  mundo aparezca antes.
- **A.** Desglose por fase (init del engine, shell RN, descarga de contenido, compilación de
  pipelines, primera escena) y atacar la mayor.
- **V.** Frío ≤12 s en A54 con mediana de 5 corridas.
- **Costo** M · **Riesgo** bajo.

**G2 · Pipelines y shaders async.**
- **E.** `async_shaders=true` ya está; el t100 de 1.x (7.7 s) contra 1.4 s de 2.x muestra cuánto pesa
  compilar en serie.
- **A.** Medir cuántos pipelines se compilan por escena y si el warm-up se puede cachear en disco por
  device.
- **V.** Segunda entrada a la misma escena sin recompilar.
- **Costo** M · **Riesgo** medio. **Depende de** G1.

### H. Shell React Native

**H1 · Tiempo a contenido por pantalla.**
- **E.** Discover 2.27 s, Backpack 1.85 s, Profile 1.16 s, Settings ≤ 0.5 s; sin fugas ni crashes.
- **A.** Donde es alto casi siempre es imagen remota: precargar el primer frame de las cards y del
  avatar, cache en disco con placeholder, y render de la lista antes de las imágenes.
- **V.** Discover ≤1 s, Backpack ≤1 s, medido con el mismo probe por capturas.
- **Costo** S/M · **Riesgo** bajo.

**H2 · Arranque del bundle y trabajo del hilo RN durante el juego.**
- **E.** El shell aporta su propio arranque al frío de G1, y su hilo puede ser el trabajo periódico de
  B1.
- **A.** Medir tiempo de bundle y trabajo del hilo RN en segundo plano mientras se juega (analytics,
  imágenes, storage); diferir lo que no sea de la ventana visible.
- **V.** Ningún trabajo del shell dentro de la ventana de juego; arranque del shell contado aparte.
- **Costo** S/M · **Riesgo** bajo. **Depende de** A4.

### I. Memoria y térmica

**I1 · Deriva de memoria en soak.**
- **E.** 71 min con +42 MB entre la primera y la última muestra, deriva suave de ~+100 MB en la
  mediana, pico 2206 MB, sin escalones.
- **A.** Soak de 20 min como test con corte por pendiente; si aparece, atribuir con `dumpsys meminfo`
  por categoría y con el cache de contenido.
- **V.** Pendiente ≤20 MB/20 min en A54.
- **Costo** S · **Riesgo** bajo.

**I2 · Política térmica con HIGH.**
- **E.** El soak corrió con los defaults (piel 38–40 °C, sin throttling); a `HIGH` la temperatura al
  cerrar la ventana sube a 44–45 °C y el arranque térmico cambia el resultado entre corridas.
- **A.** Medir una sesión larga a `HIGH` (30 min) para saber si el device eventualmente throttlea y
  cómo se comporta el cap térmico (`thermal_fps_cap_enabled`).
- **V.** Curva de fps vs temperatura en 30 min a HIGH; decisión sobre el cap térmico por tier.
- **Costo** M · **Riesgo** bajo.

### J. Política de calidad y perfiles

**J1 · Target de dynamic graphics por tier.**
- **E.** El objetivo es 16.6 ms (60 fps) y el resultado ~18 fps; con HIGH y cap apagado el mismo
  teléfono hace ~31 fps. El objetivo declarado y la capacidad real no coinciden, y el perfil oscila
  sin que la decisión quede registrada.
- **A.** Target por tier de dispositivo, histéresis para no oscilar y log de cada decisión con motivo.
- **V.** Con dynamic ON: dispersión de p50 dentro del run y cantidad de cambios de perfil por minuto,
  antes/después.
- **Costo** S · **Riesgo** bajo técnico, alto de producto (cambia lo que ve el usuario).

**J2 · Perfiles por device tier.**
- **E.** Todo este benchmark está medido en un solo device (A54, Adreno). No se sabe si el orden de
  los perfiles se sostiene en un Mali de gama baja.
- **A.** Repetir la matriz en 2–3 devices (gama baja, media, flagship) y ajustar los presets por tier
  en vez de por modelo.
- **V.** Misma tabla en tres devices.
- **Costo** M · **Riesgo** bajo.

### K. Paridad y plataforma

**K1 · iOS medible.**
- **E.** La inyección de input está resuelta (Appium + WebDriverAgent en el iPhone 16e) y el HUD del
  engine **sí** aparece en el árbol de accesibilidad: `iphone ui` devuelve
  `2.0.0.2723-e3b7aa1-staging - 60 FPS`. Es un contador a 1 Hz, no una frame timeline, pero es una
  fuente real y no requiere tocar el cliente. Primera muestra: 59–60 fps en Bloom Garden con los
  ajustes que se entregan.
- **A.** Envolver el muestreo en un script (N muestras, mediana, escena y ajustes anotados) y correr
  el mismo escenario que en Android; para p50/p99 sigue haciendo falta A4.
- **V.** fps de iOS en el reporte con su método declarado (muestreo del HUD, no frametimeline).
- **Costo** S · **Riesgo** bajo. **Estado:** la fuente existe y está probada; falta el escenario
  pareado y, si se quieren colas, A4.

**K2 · Paridad de métricas entre plataformas.**
- **E.** Android mide memoria con PSS y térmica con `thermalservice`; iOS no tiene equivalente
  cableado. Un número de iOS sin su contexto es peor que no tenerlo.
- **A.** Definir qué se reporta en cada plataforma y con qué instrumento, antes de publicar números
  cruzados.
- **V.** Una tabla por plataforma donde cada columna existe en las dos.
- **Costo** S · **Riesgo** bajo. **Depende de** K1.

---

## 3. Targets propuestos (SLO de benchmark)

Medidos con la condición de A1/A2 (cielo fijo verificado por luma, ventana de 30 s, arranque térmico ≤34 °C, mediana de 3).

| Escenario | Hoy | Target |
|---|---|---|
| `gp` noon, HIGH, dynamic OFF | 2.x 30.5–31.3 fps · 1.x a fijar | p50 ≤ 30 ms, p99 ≤ 45 ms, ningún frame > 66 ms |
| `gp` dusk (equivalente) | 2.x ~19.8 fps | ≤10 % peor que noon |
| Arranque en frío | 19.4 s | ≤12 s |
| Entrar/salir ×20 | 8.5 s + 1680 MB | ≤7 s, sin crecimiento entre ciclo 1 y 20 |
| Soak 20 min | +42 MB/71 min | pendiente ≤20 MB/20 min, sin throttling |
| Shell RN Discover/Backpack | 2.27 s / 1.85 s | ≤1 s cada uno |

Los targets son propuestas a discutir, no compromisos: se fijan después de A4, cuando el número sea
barato de medir.

---

## 4. Lo que no priorizaría todavía

- **Micro-optimizar el frame medio.** Con dynamic ON empatan por diseño; con el cap apagado 2.x ya
  duplica a 1.x. No hay problema demostrado en la mediana.
- **Memoria.** 2.x usa menos en todos los estados medidos y no fuga en 20 ciclos. Sin problema
  demostrado, optimizarla es trabajo sin métrica (la excepción es la deriva del soak, I1).
- **Bajar calidad por defecto para ganar fps.** El problema no es el techo del render sino qué decide
  el techo (J1) y la cola (B1). Bajar calidad esconde el síntoma.
- **Comparar fps entre builds con la configuración que se entrega hoy.** Mide el cap de dynamic
  graphics (§1 del reporte) y ahora sabemos que además mide la hora del mundo.

---

## 5. Secuencia

```
A1 (el ajuste de cielo llega al motor) ──> A2 (condiciones) ─┬─> A6 (fingerprint) ──> A5 (CI)
                                                             └─> A3 (deeplink) ──> D1 (avatares) ──> D2/D3
A4 (contadores) ──┬─> B1 (hitches) ──> B2 (presupuesto)
                  ├─> C1 (día/noche) ──> C2 (luces por perfil)
                  ├─> E1 (GPU vs CPU) ──> E2 / E3
                  ├─> F1/F2 (runtime de escena)
                  └─> K1 (iOS)
G1 (arranque) ──> G2 (pipelines)          H1/H2 (shell RN)          I1/I2 (memoria y térmica)
J1 (target por tier) ──> J2 (perfiles por device)
```

**Ahora (esta semana):** A1 (hecho), A5 (cerrar), A2, A3. Baratos y desbloquean el resto.
**Siguiente:** B1 + C1 + E1 — las tres preguntas que la medición ya abrió; con sus respuestas se
decide si el trabajo grande está en el runtime, en la iluminación o en el render.
**Después:** D1/D2, G1, H1, I1 en paralelo por áreas distintas.
**Largo plazo:** A4 (CI), J2 (más devices), E2 si E1 dice que el frame es GPU-bound.

---

## 6. Riesgos transversales

- **Optimizar contra la métrica equivocada.** Todo esto asume que fps/p99 bajo condiciones fijas
  representan la experiencia. Si el usuario juega de noche en un mundo distinto, la prioridad cambia:
  por eso C1 y A1 están arriba.
- **Regresión visual silenciosa.** Los ítems que tocan calidad (C2, D2, E3, J1) necesitan aprobación
  con capturas, no sólo con fps.
- **Un solo device.** Todas las conclusiones son del A54/Adreno. J2 existe para eso.
- **El instrumento se vuelve el producto.** A3/A4 son medios; si el gate de CI no se pone en `fail`
  en un mes, es señal de que los umbrales están mal, no de que el gate sobre.

---

## 7. Cómo se mide

```bash
# una corrida: enfría el teléfono a ≤34 °C, lanza el mundo y mide 30 s
# verifica perfil cargado por el engine, ausencia de cambios de perfil y luma de la ventana
WINDOW=30 SETTLE=25 TARGET_TEMP=34 SCENARIO=gp-hi-fixed-sky \
  scripts/bench/2x-vs-1x/run_paired.sh 2x 2x-sky-r1

# tabla agregada desde los CSV crudos (saltea escenarios con INVALID.md)
python3 scripts/bench/2x-vs-1x/summarize.py --root docs/bench/data/2x-vs-1x-3206

# el sol no se puede fijar desde el cliente: se muestrea una build mientras la luz se mueve
BUILD=2x LABEL=2x-drift SAMPLES=12 GAP=75 scripts/bench/2x-vs-1x/run_drift.sh

# iOS: fps del HUD del engine vía árbol de accesibilidad (1 Hz, sin frame timeline)
SETTLE=25 SAMPLES=20 scripts/bench/2x-vs-1x/ios_hud_sample.sh 2x-ios-r1
```

Cada corrida deja: `frames.csv`, `cpu.csv`, `mem.csv`, `thermal.csv`, `meminfo-*.log`,
`trace.perfetto-trace`, `launch.log` (filtrado por pid), `state.json` (luma + temperaturas) y
`screen.png`.
