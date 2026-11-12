# 07 — Plan final tras la revisión y la reconstrucción offline

Este documento reemplaza al `04-campaign-plan.md` donde los dos revisores y la reconstrucción offline lo
corrigen. Fuentes: `05-review.md` (revisión hostil), `06-risks.md` (segunda opinión), y el resultado de
`/tmp/plan-3206/reconstructed-summary.md`, que **recalculó 57 directorios de corrida ya existentes sin
tocar el teléfono**.

## 1. Qué cambió por la reconstrucción offline

Se derivaron de los datos ya guardados la cadencia (`frames.csv`), la luma central (`screen.png`,
recorte 70 %×70 % para excluir el HUD) y la estructura de stalls, para **57 corridas**. Tres
consecuencias:

1. **La métrica primaria cambia.** El plan proponía `pct_window_in_stalls`; su CV dentro de brazo es
   68.6 % de mediana (peor 198.6 %), porque los stalls son ~5 eventos por ventana. Con eso, n=6 sólo
   detectaría una diferencia del ~73 %. La métrica con menor CV es **`pct_frames_on_modal`** (CV mediana
   **11.2 %** sobre 8 brazos con n>=3, peor 37.2 %), o sea **MDR 18.2 % con n=6 por brazo**. Se
   pre-registra ésa; el resto queda como secundaria/diagnóstica.
2. **Un brazo del plan ya existía.** `gp-hi/2x-gp-hi-r1..r5` es la condición P1 en 2.x (perfil 3,
   confirmado en los logs a nivel escenario) y sólo le faltaban `state.json` y `launch.log` por corrida:
   ambos reconstruibles. Con luma central 69.8–90.6 y fps mediana 14.99 **no hay que volver a medirlo**.
3. **S2 (sensibilidad al sol) ya está medido.** `gp-sun-drift` tiene 12 ventanas de 2.x con luma
   53.9–60.9 y fps 18.38–22.89, CPU 174–201 %. No se re-mide; se cita.

## 2. La condición que sí es defendible

El oráculo identificó el error de fondo del plan: en Genesis Plaza la escena fija su propio reloj, así
que **no existe** una condición donde el sol quede igualado por ajuste. La única igualdad estructural
disponible sin tocar el cliente es el **reloj del mundo**:

```
world_time = (utc_unix_seconds × 12) mod 86400      # 120 min reales por día del juego
```

Con `Dynamic Skybox` ON, las dos builds calculan el sol con esa fórmula
(`lib/crates/dcl_engine/src/godot_classes/dcl_global_time.rs`), el período es **7200 s reales**, y por lo
tanto **arrancando las dos ramas en la misma fase UTC el sol es el mismo por construcción**. No hace
falta la luma como variable de igualación (que el propio `03-tooling.md` §4.3 declara confundida entre
builds por el HUD distinto).

Y hace falta una parcela **sin escena** (`100,100`) para que ninguna escena se quede con el reloj. Con
`Dynamic Skybox` OFF no se mide nunca: es el único ajuste que manda a 2.x a medianoche por el
`try_to::<f64>()` estricto sobre un variante `Int`.

## 3. La campaña

| brazo | condición | corridas | ventana | para qué |
|---|---|---:|---|---|
| **P** | parcela `100,100`, HIGH, dynamic graphics OFF, `Dynamic Skybox` ON, arranque en fase UTC igualada | 6 + 6 | 30 s | la comparación entre builds, con sol igualado por construcción |
| **N** | parcela `100,100`, 2.x contra 2.x, misma fase, mismo protocolo | 3 + 3 | 30 s | control nulo: mide el sesgo del arnés, la térmica y el orden |
| **G** | Genesis Plaza, HIGH, dynamic graphics OFF, `Dynamic Skybox` ON | 3 + 3 | 30 s | descriptivo: dos bandas, **sin delta** |

- **Total: 24 corridas nuevas**, ~10.4 min cada una con compuerta térmica ⇒ **≈ 4.2 h**, más 2
  instalaciones (8 min) y el preflight (40 min) ⇒ **≈ 5 h**, partidas en dos bloques.
- **Lo que se corta primero**: G (descriptivo, −62 min). Después N a 2+2 (−42 min). **Piso: P a 4+4**;
  por debajo, se publican bandas y no se da delta.
- **Preflight que puede abortar todo**: F0 batería (no se arranca por debajo de 40 %), F1 térmica
  (status ≤ 1, AP ≤ 40 °C), F2 la herramienta sobre una corrida histórica conocida, F3 fase UTC
  igualada dentro de 2 s, F4 que las dos builds lean el mismo perfil 3 del log, F5 que la luma central
  del mismo brazo no se mueva más de 3 puntos entre dos corridas seguidas.

## 4. Bloqueos operativos (medidos hoy)

| bloqueo | estado | qué hace falta |
|---|---|---|
| Batería | **10 %**, `status: 3` (descargando), USB sin alimentación | cargar a ≥40 % antes de cualquier corrida; el teléfono debe quedar enchufado entre bloques |
| Térmica | `Thermal Status: 2` (MODERATE), AP 48.9 °C en reposo | enfriar hasta status ≤ 1 y AP ≤ 40 °C; la compuerta exige además 480 s desde la ventana anterior |
| Panel | `peak_refresh_rate` = `null` | `display_mode.sh pin` a 120.0 y verificación desde `dumpsys display` en cada corrida |
| 2.x skybox | roto con `Dynamic Skybox` OFF (`Int` vs `f64`) | no se arregla acá; se evita esa condición en los brazos medidos y queda como ítem A1 del roadmap |

## 5. Reglas de invalidación (aplicables a una corrida terminada, sin repetirla)

1. Perfil distinto de 3 en el `[RenderReady]` del pid propio, o una línea `Profile (downgrade|upgrade)`
   dentro de la ventana.
2. Modal de intervalo que no cae en múltiplos del período de 120 Hz, o dos modos con más del 2 % fuera
   del modal (refresh adaptativo).
3. Menos de 480 s desde el fin de la ventana anterior.
4. `since_prev_end_s` ausente en el ledger de la compuerta térmica.
5. Luma central fuera de la banda del brazo en 2 o más corridas de un bloque de 6.
6. Cualquier `INVALID.md` propio o de un ancestro.
7. `meta.json` sin `frames.fps_avg`, o `frames.csv` con 0 filas.
8. Fase UTC de arranque distinta de la objetivo en más de 2 s (sólo brazo P).

## 6. Qué se publica

- **Delta entre builds** sólo si P produce un par con fase igualada y las 6+6 corridas pasan las reglas.
  Estadístico: diferencia de medianas de `pct_frames_on_modal` con IC bootstrap (10 000 remuestreos) y
  test exacto de Mann–Whitney como control. Con n=6, el piso bilateral exacto es 0.0022 y el MDR 18.2 %.
- **Control nulo**: si N muestra una diferencia entre dos corridas 2.x mayor que la mitad del efecto
  observado en P, el delta de P se declara no concluyente y se publican bandas.
- **Genesis Plaza**: dos bandas con su luma central, sin delta, y la frase explícita de que el sol no se
  puede igualar ahí.
- **Nada de**: atribución causal al shell React Native, tiempo de GPU, p99.9 desde 30 s, mezcla
  Android+iOS, ni extrapolación a otro device.
