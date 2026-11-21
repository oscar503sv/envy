# Envy v0.1.0 — Plan de sprints

> **Estado:** plan provisional; las fechas son **estimadas**, no compromisos aprobados.  
> **Fuente de tareas:** [BACKLOG.md](BACKLOG.md). Hitos en [MILESTONES.md](MILESTONES.md).

## 1. Supuestos de capacidad

| Parámetro | Valor | Estado |
|---|---|---|
| Fecha de inicio | 2026-11-23 (lunes) | Dato del encargo |
| Duración de sprint | 1 semana, lunes a viernes | Dato del encargo |
| Equipo | 1 desarrollador + agentes de IA (sin paralelismo humano) | Dato del encargo |
| Metodología | Scrumban ligero: planificación semanal + flujo Kanban con WIP = 1 tarea de implementación | Dato del encargo / propuesta WIP |
| Capacidad bruta | 41 h/semana | **Confirmada por el usuario** (comunicado por el coordinador, 2026-11-20) — [D-02](DECISIONS.md#d-02) resuelta |
| Reserva | 20 % de la capacidad bruta para coordinación, revisión de la salida de los agentes, planificación semanal y contingencias | **Supuesto** (se mantiene; ver [D-35](DECISIONS.md#d-35)) |
| Capacidad neta planificable | 32.8 h/semana | Derivado |
| Semanas festivas | S05 (25-dic) y S06 (1-ene): se descuenta 1 día => 32.8 h brutas / 26.24 h netas | **Supuesto** ([D-03](DECISIONS.md#d-03)) |
| Otros festivos/vacaciones | No considerados (el plan termina antes de Semana Santa 2027) | **Supuesto** — riesgo R-03 |

Las estimaciones de cada tarea ya incluyen sus pruebas unitarias, revisión e integración. Además hay tareas explícitas de integración/corrección (ENV-T053, ENV-T063, ENV-T075, ENV-T086, ENV-T088) y de verificación transversal (épica E8).

## 2. Verificación aritmética

- Suma de estimaciones del backlog: **417 h** en **88 tareas**.
- Suma de horas asignadas a sprints: **417 h** (debe coincidir con el backlog: sí).
- Capacidad neta total en 15 sprints: **478.88 h** · utilización planificada **87.1 %** · holgura **61.88 h**.
- Capacidad bruta total: 598.6 h (incluye la reserva del 20 %).
- Ningún sprint supera su capacidad neta y ninguna tarea se planifica antes que sus dependencias (dentro de un sprint, las tareas se ejecutan en el orden listado). Validado con un script de comprobación sobre los datos del backlog.
- Duración resultante: 15 semanas de calendario, de 2026-11-23 a 2027-03-05. Se deriva de 417 h / 32.8 h netas por semana, del empaquetado por dependencias y de las puertas de aprobación; no se ha forzado a un número de semanas.
- **Puertas de aprobación:** las tareas que dependen de una propuesta que el usuario debe aprobar (spikes y especificación) se planifican en un sprint **posterior** al de la tarea que la produce: ENV-T006 → ENV-T010, ENV-T011, ENV-T014; ENV-T001 → ENV-T033; ENV-T025 → ENV-T027, ENV-T028; ENV-T054 → ENV-T055, ENV-T077; ENV-T039 → ENV-T037, ENV-T046. Verificado por el mismo script.
- **Estimaciones:** no se ha modificado ninguna estimación al pasar de 15 a 41 h/semana; el cambio de capacidad no altera el trabajo técnico de cada tarea.

## 3. Resumen de sprints

| Sprint | Fechas | Fases | Nombre | Bruta | Neta | Plan | Holgura | Tareas (en orden) |
|---|---|---|---|---|---|---|---|---|
| S01 | 2026-11-23 → 2026-11-27 | A | Arranque, especificación y modelo | 41 | 32.8 | 31 | 1.8 | ENV-T001, ENV-T002, ENV-T006, ENV-T004, ENV-T007, ENV-T008, ENV-T009, ENV-T024 |
| S02 | 2026-11-30 → 2026-12-04 | A | Spike de guardado y parser | 41 | 32.8 | 31 | 1.8 | ENV-T025, ENV-T003, ENV-T010, ENV-T011, ENV-T012, ENV-T013 |
| S03 | 2026-12-07 → 2026-12-11 | A | Spike Flatpak, ValueCodec y SetValue | 41 | 32.8 | 30 | 2.8 | ENV-T054, ENV-T014, ENV-T015, ENV-T020, ENV-T022, ENV-T023, ENV-T016 |
| S04 | 2026-12-14 → 2026-12-18 | A/B | Historial, almacenamiento y sesión | 41 | 32.8 | 31 | 1.8 | ENV-T017, ENV-T018, ENV-T019, ENV-T026, ENV-T027, ENV-T029 |
| S05 | 2026-12-21 → 2026-12-25 (festivo) | B | Guardado condicionado y monitor (semana festiva) | 32.8 | 26.24 | 24 | 2.24 | ENV-T028, ENV-T030, ENV-T031, ENV-T067 |
| S06 | 2026-12-28 → 2027-01-01 (festivo) | B | Pegado, conflictos y regresión del núcleo (semana festiva) | 32.8 | 26.24 | 26 | 0.24 | ENV-T032, ENV-T021, ENV-T070, ENV-T069, ENV-T068 |
| S07 | 2027-01-04 → 2027-01-08 | C | Arranque de la UI y EditorView | 41 | 32.8 | 30 | 2.8 | ENV-T033, ENV-T034, ENV-T036, ENV-T035, ENV-T039, ENV-T041 |
| S08 | 2027-01-11 → 2027-01-15 | C | Edición, guardado y conflictos en UI | 41 | 32.8 | 31 | 1.8 | ENV-T047, ENV-T037, ENV-T040, ENV-T038, ENV-T042, ENV-T045 |
| S09 | 2027-01-18 → 2027-01-22 | C | Undo/redo, secretos, copia, cierre y pegado | 41 | 32.8 | 31 | 1.8 | ENV-T046, ENV-T043, ENV-T048, ENV-T049, ENV-T044, ENV-T050, ENV-T051, ENV-T052 |
| S10 | 2027-01-25 → 2027-01-29 | C/D | Integración Quick Edit y catálogo | 41 | 32.8 | 31 | 1.8 | ENV-T005, ENV-T053, ENV-T055, ENV-T071, ENV-T056, ENV-T057 |
| S11 | 2027-02-01 → 2027-02-05 | D | Navegación del Workspace | 41 | 32.8 | 31 | 1.8 | ENV-T059, ENV-T058, ENV-T066, ENV-T060, ENV-T061, ENV-T074, ENV-T062, ENV-T072 |
| S12 | 2027-02-08 → 2027-02-12 | D/E | Herramientas de entorno, accesibilidad y Flatpak | 41 | 32.8 | 29 | 3.8 | ENV-T064, ENV-T065, ENV-T063, ENV-T073, ENV-T077 |
| S13 | 2027-02-15 → 2027-02-19 | E | Metadatos, i18n y validación Flatpak | 41 | 32.8 | 32 | 0.8 | ENV-T078, ENV-T080, ENV-T079, ENV-T081, ENV-T082, ENV-T086 |
| S14 | 2027-02-22 → 2027-02-26 | E/F | Endurecimiento, documentación y RC | 41 | 32.8 | 18 | 14.8 | ENV-T075, ENV-T083, ENV-T084, ENV-T085 |
| S15 | 2027-03-01 → 2027-03-05 | F | Regresión RC y release | 41 | 32.8 | 11 | 21.8 | ENV-T076, ENV-T088, ENV-T087 |
| | | | **Total** | **598.6** | **478.88** | **417** | **61.88** | 88 tareas |

## 4. Detalle por sprint

### Fase A — Fundación y motor de documentos

#### S01 — Arranque, especificación y modelo

- **Fechas:** 2026-11-23 → 2026-11-27.
- **Capacidad:** 41 h brutas / 32.8 h netas · **planificado:** 31 h · holgura 1.8 h.
- **Objetivo:** Validar el stack, fijar la especificación de sintaxis y tener modelo, fixtures y segmentación de líneas.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T001 Spike: validar toolchain y compatibilidad gtk4-rs/libadwaita/runtime GNOME | 4 | P0 | Ninguna |
| 2 | ENV-T002 Crear el esqueleto del crate con módulos por capas | 3 | P0 | Ninguna |
| 3 | ENV-T006 Redactar la especificación de sintaxis dotenv v0.1 | 5 | P0 | Ninguna |
| 4 | ENV-T004 Definir tipos de error y política de diagnóstico sin secretos | 4 | P0 | ENV-T002 |
| 5 | ENV-T007 Crear el corpus de fixtures de fidelidad | 4 | P0 | ENV-T006 |
| 6 | ENV-T008 Implementar el modelo EnvDocument/EnvNode con spans absolutos | 4 | P0 | ENV-T002, ENV-T006, ENV-T004 |
| 7 | ENV-T009 Implementar segmentación de líneas, BOM y compuerta UTF-8 | 5 | P0 | ENV-T008 |
| 8 | ENV-T024 Implementar la inferencia de etiqueta de entorno por nombre de archivo | 2 | P2 | ENV-T002 |

- **Dependencias de sprints anteriores:** Ninguna.
- **Entregables:** Propuesta de versiones/sistema de build; crate con módulos; docs/SYNTAX.md con preguntas abiertas (D-11); errores redactados; fixtures binario-exactos; modelo EnvDocument; lexer de líneas; inferencia de etiquetas.
- **Criterios de aceptación del sprint:** cargo build/test/fmt/clippy en verde; tabla de versiones propuesta; especificación revisada contra PRD/TD; tests de redacción, invariantes y lexer en verde; fixtures intactos tras clon limpio. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** Sprint muy cargado de arranque; D-11 debe aprobarse antes de que ENV-T010 empiece en S02; disponibilidad del SDK Flatpak para el spike.
- **Demo esperada:** Ventana Adw mínima del spike; recorrido de SYNTAX.md; tests del lexer sobre mixed-newlines.env.

#### S02 — Spike de guardado y parser

- **Fechas:** 2026-11-30 → 2026-12-04.
- **Capacidad:** 41 h brutas / 32.8 h netas · **planificado:** 31 h · holgura 1.8 h.
- **Objetivo:** Conocer la semántica real del guardado Gio y demostrar fidelidad byte a byte del parser completo.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T025 Spike: semántica real de guardado condicionado con Gio en sistema local | 6 | P0 | ENV-T001, ENV-T002 |
| 2 | ENV-T003 Configurar CI del núcleo con fmt, clippy y tests | 4 | P0 | ENV-T002 |
| 3 | ENV-T010 Implementar el reconocimiento de asignaciones de una línea | 7 | P0 | ENV-T009, ENV-T007 |
| 4 | ENV-T011 Implementar valores multilínea entre comillas | 6 | P0 | ENV-T010 |
| 5 | ENV-T012 Implementar el serializador puro y el arnés de fidelidad byte a byte | 4 | P0 | ENV-T010, ENV-T011, ENV-T007 |
| 6 | ENV-T013 Añadir pruebas de propiedades del roundtrip parser/serializador | 4 | P0 | ENV-T012 |

- **Dependencias de sprints anteriores:** ENV-T001, ENV-T002, ENV-T007, ENV-T009.
- **Entregables:** Matriz del spike de guardado (propuesta D-12); CI del núcleo; parser de una línea y multilínea; serializador; fidelidad 100 %; proptest.
- **Criterios de aceptación del sprint:** Matriz del spike completa; CI en verde y falla ante warning; serialize(parse(b)) == b para el 100 % de fixtures; proptest sin fallos. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** El orden (spike y CI primero) da 1-2 días de margen para aprobar D-11 antes de ENV-T010; R-04 en multilínea.
- **Demo esperada:** tests/fidelity.rs y proptest en verde; tabla de resultados del spike Gio.

#### S03 — Spike Flatpak, ValueCodec y SetValue

- **Fechas:** 2026-12-07 → 2026-12-11.
- **Capacidad:** 41 h brutas / 32.8 h netas · **planificado:** 30 h · holgura 2.8 h.
- **Objetivo:** Eliminar la incertidumbre de Flatpak y poder editar valores con reemplazo mínimo.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T054 Spike: acceso a carpetas, persistencia de permisos y CLI en Flatpak | 8 | P0 | ENV-T001, ENV-T025 |
| 2 | ENV-T014 Implementar ValueCodec por estilo de comillas | 7 | P0 | ENV-T006, ENV-T011 |
| 3 | ENV-T015 Crear el ejemplo interno roundtrip de verificación de fidelidad | 2 | P1 | ENV-T012, ENV-T004 |
| 4 | ENV-T020 Implementar el diagnóstico de claves duplicadas | 2 | P1 | ENV-T010 |
| 5 | ENV-T022 Implementar la heurística de secretos por nombre de clave | 2 | P1 | ENV-T008 |
| 6 | ENV-T023 Implementar la comparación de conjuntos de claves entre documentos | 3 | P0 | ENV-T010 |
| 7 | ENV-T016 Implementar el comando SetValue con reemplazo mínimo | 6 | P0 | ENV-T014, ENV-T012 |

- **Dependencias de sprints anteriores:** ENV-T001, ENV-T004, ENV-T006, ENV-T008, ENV-T010, ENV-T011, ENV-T012, ENV-T025.
- **Entregables:** Matriz del spike Flatpak (propuesta D-19); ValueCodec; ejemplo roundtrip; duplicados; heurística de secretos; comparación de claves; SetValue.
- **Criterios de aceptación del sprint:** Resultado documentado del spike en sandbox; encode(decode(v)) reproduce bytes editables; SetValue deja intactos los bytes fuera del valor. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** Resultado negativo del spike Flatpak (R-02) exige decisión antes de S10; R-04 en el codec.
- **Demo esperada:** Prototipo Flatpak reabriendo una carpeta tras reiniciar (o evidencia del fallo); edición de DEBUG con comparación de bytes.

#### S04 — Historial, almacenamiento y sesión

- **Fechas:** 2026-12-14 → 2026-12-18.
- **Fases cubiertas:** Fase A — Fundación y motor de documentos, Fase B — Persistencia segura y conflictos (núcleo).
- **Capacidad:** 41 h brutas / 32.8 h netas · **planificado:** 31 h · holgura 1.8 h.
- **Objetivo:** Cerrar el motor de edición (M1) y comenzar la persistencia: lectura segura y sesión de documento.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T017 Implementar el comando InsertAssignment al final del documento | 5 | P1 | ENV-T012 |
| 2 | ENV-T018 Implementar el comando RemoveAssignment de una ocurrencia | 3 | P1 | ENV-T012 |
| 3 | ENV-T019 Implementar EditHistory con lotes transaccionales y cálculo de dirty | 6 | P0 | ENV-T016, ENV-T017, ENV-T018 |
| 4 | ENV-T026 Definir el puerto DocumentStorage y un almacenamiento falso para pruebas | 4 | P0 | ENV-T008, ENV-T004 |
| 5 | ENV-T027 Implementar la lectura segura de archivos en GioStorage | 6 | P0 | ENV-T026, ENV-T025 |
| 6 | ENV-T029 Implementar DocumentSession y la máquina de estados del documento | 7 | P0 | ENV-T019, ENV-T026 |

- **Dependencias de sprints anteriores:** ENV-T004, ENV-T008, ENV-T012, ENV-T016, ENV-T025.
- **Entregables:** InsertAssignment, RemoveAssignment, EditHistory; DocumentStorage + FakeStorage; lectura GioStorage; DocumentSession.
- **Criterios de aceptación del sprint:** Undo/redo de lotes exacto; errores de lectura distintos por caso; cada fila de TD §7.2 con test. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** D-25 (hash) necesaria al inicio del sprint.
- **Demo esperada:** Secuencia apply/undo/redo con lote; tests de lectura con tempfile; transiciones con FakeStorage. Hito M1.

### Fase B — Persistencia segura y conflictos (núcleo)

#### S05 — Guardado condicionado y monitor (semana festiva)

- **Fechas:** 2026-12-21 → 2026-12-25. Festivo supuesto: 2026-12-25 (viernes).
- **Capacidad:** 32.8 h brutas / 26.24 h netas · **planificado:** 24 h · holgura 2.24 h.
- **Objetivo:** Guardar sin sobrescrituras silenciosas y detectar cambios externos, con capacidad reducida por el 25-dic.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T028 Implementar replace_if_unchanged con verificación posterior | 7 | P0 | ENV-T027, ENV-T025 |
| 2 | ENV-T030 Implementar la orquestación de Guardar y Guardar como | 5 | P0 | ENV-T029, ENV-T028 |
| 3 | ENV-T031 Implementar el adaptador FileMonitor con coalescencia y verificación por hash | 6 | P0 | ENV-T029, ENV-T027 |
| 4 | ENV-T067 Crear la matriz de pruebas de integración de guardado seguro | 6 | P0 | ENV-T028 |

- **Dependencias de sprints anteriores:** ENV-T025, ENV-T027, ENV-T029.
- **Entregables:** replace_if_unchanged; Guardar/Guardar como; FileMonitor; matriz de guardado seguro.
- **Criterios de aceptación del sprint:** Conflicto ante cambio externo; éxito solo tras verificación; eventos coalescidos; matriz completa en CI. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** D-12, D-27 y D-34 deben estar decididas el 2026-12-21; R-01, R-15; disponibilidad del usuario en festivos.
- **Demo esperada:** Script que modifica el archivo entre lectura y guardado y muestra el rechazo.

#### S06 — Pegado, conflictos y regresión del núcleo (semana festiva)

- **Fechas:** 2026-12-28 → 2027-01-01. Festivo supuesto: 2027-01-01 (viernes).
- **Capacidad:** 32.8 h brutas / 26.24 h netas · **planificado:** 26 h · holgura 0.24 h.
- **Objetivo:** Completar la lógica de núcleo (M2) con pegado, revisión de conflicto y suites de regresión.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T032 Implementar el resumen de revisión de conflicto sin valores | 4 | P0 | ENV-T029, ENV-T023 |
| 2 | ENV-T021 Implementar el núcleo de pegado de asignaciones (PastePlan) | 7 | P1 | ENV-T011, ENV-T019, ENV-T020 |
| 3 | ENV-T070 Auditar y probar la ausencia de secretos en logs y errores del núcleo | 4 | P0 | ENV-T030, ENV-T021 |
| 4 | ENV-T069 Crear las pruebas de integración de conflictos y monitorización | 6 | P0 | ENV-T030, ENV-T031 |
| 5 | ENV-T068 Crear la suite de regresión de edición | 5 | P0 | ENV-T019, ENV-T021 |

- **Dependencias de sprints anteriores:** ENV-T011, ENV-T019, ENV-T020, ENV-T023, ENV-T029, ENV-T030, ENV-T031.
- **Entregables:** Resumen de conflicto; PastePlan; auditoría de fugas del núcleo; tests de conflictos y de edición.
- **Criterios de aceptación del sprint:** Ningún test permite pérdida sin acción expresa; centinela ausente; reglas de pegado de US-03 probadas. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** Sprint al límite de capacidad (holgura 0,24 h) en semana festiva.
- **Demo esperada:** Suite completa del núcleo en CI. Hito M2.

### Fase C — Quick Edit

#### S07 — Arranque de la UI y EditorView

- **Fechas:** 2027-01-04 → 2027-01-08.
- **Capacidad:** 41 h brutas / 32.8 h netas · **planificado:** 30 h · holgura 2.8 h.
- **Objetivo:** Abrir archivos desde terminal y visualizarlos con filas por tipo de nodo y sesiones independientes.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T033 Inicializar la aplicación GTK4/libadwaita y la ventana principal | 6 | P0 | ENV-T001, ENV-T002, ENV-T004 |
| 2 | ENV-T034 Implementar la CLI y el manejo de apertura de archivos | 5 | P1 | ENV-T033, ENV-T027 |
| 3 | ENV-T036 Implementar EditorView con filas por tipo de nodo | 7 | P0 | ENV-T033, ENV-T029 |
| 4 | ENV-T035 Implementar el registro de sesiones y ventanas por documento | 4 | P1 | ENV-T034, ENV-T029 |
| 5 | ENV-T039 Spike: política única de undo y granularidad de confirmación de edición | 4 | P1 | ENV-T036 |
| 6 | ENV-T041 Implementar indicadores de estado y modos de solo lectura | 4 | P1 | ENV-T036, ENV-T029 |

- **Dependencias de sprints anteriores:** ENV-T001, ENV-T002, ENV-T004, ENV-T027, ENV-T029.
- **Entregables:** AdwApplication y ventana; CLI; EditorView; registro de sesiones; propuesta D-13; indicadores de estado.
- **Criterios de aceptación del sprint:** envy .env abre el archivo; todos los fixtures visibles con su tipo de fila; estados con texto e icono. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** D-05, D-06, D-09, D-15, D-28, D-33 (y D-07 recomendada) decididas el 2027-01-04; curva de gtk-rs (R-05).
- **Demo esperada:** envy fixtures/duplicate-keys.env desde terminal; estado Solo lectura y UTF-8 inválido.

#### S08 — Edición, guardado y conflictos en UI

- **Fechas:** 2027-01-11 → 2027-01-15.
- **Capacidad:** 41 h brutas / 32.8 h netas · **planificado:** 31 h · holgura 1.8 h.
- **Objetivo:** Editar (simple y multilínea), guardar y resolver conflictos desde la UI.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T047 Implementar la búsqueda de variables por clave | 4 | P1 | ENV-T036 |
| 2 | ENV-T037 Implementar la edición de valores de una línea con conversión explícita | 6 | P0 | ENV-T036, ENV-T039, ENV-T016, ENV-T014 |
| 3 | ENV-T040 Implementar las acciones Guardar y Guardar como en la UI | 5 | P0 | ENV-T036, ENV-T030 |
| 4 | ENV-T038 Implementar la fila expandible para valores multilínea | 5 | P0 | ENV-T037 |
| 5 | ENV-T042 Implementar el banner de conflicto y sus flujos de resolución | 6 | P0 | ENV-T040, ENV-T031, ENV-T032 |
| 6 | ENV-T045 Implementar agregar y eliminar asignaciones en la UI | 5 | P1 | ENV-T036, ENV-T017, ENV-T018 |

- **Dependencias de sprints anteriores:** ENV-T014, ENV-T016, ENV-T017, ENV-T018, ENV-T030, ENV-T031, ENV-T032, ENV-T036, ENV-T039.
- **Entregables:** Búsqueda; edición con conversión explícita; Guardar/Guardar como; fila multilínea; banner de conflicto; agregar/eliminar.
- **Criterios de aceptación del sprint:** Editar y guardar cambia solo esos bytes; conflicto persistente con sus tres acciones; Ctrl+N y eliminar funcionan. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** D-13 y D-14 decididas el 2027-01-11.
- **Demo esperada:** Situación A parcial y Situación D con edición externa.

#### S09 — Undo/redo, secretos, copia, cierre y pegado

- **Fechas:** 2027-01-18 → 2027-01-22.
- **Capacidad:** 41 h brutas / 32.8 h netas · **planificado:** 31 h · holgura 1.8 h.
- **Objetivo:** Completar todas las interacciones de Quick Edit.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T046 Integrar undo/redo del historial en la UI | 4 | P1 | ENV-T039, ENV-T019, ENV-T045, ENV-T037 |
| 2 | ENV-T043 Implementar el aviso de enlace simbólico y la selección explícita del destino | 3 | P0 | ENV-T034, ENV-T027 |
| 3 | ENV-T048 Implementar la ocultación visual de secretos con revelado por fila | 5 | P1 | ENV-T036, ENV-T022, ENV-T038 |
| 4 | ENV-T049 Implementar copiar valor y copiar documento | 3 | P1 | ENV-T036, ENV-T014 |
| 5 | ENV-T044 Implementar la protección al cerrar con cambios sin guardar | 4 | P0 | ENV-T040, ENV-T035 |
| 6 | ENV-T050 Implementar el diálogo de pegado con previsualización | 5 | P1 | ENV-T021, ENV-T046 |
| 7 | ENV-T051 Implementar la resolución de duplicados y líneas inválidas en el pegado | 4 | P1 | ENV-T050 |
| 8 | ENV-T052 Implementar la pantalla de bienvenida y la acción Abrir archivo | 3 | P1 | ENV-T034 |

- **Dependencias de sprints anteriores:** ENV-T014, ENV-T019, ENV-T021, ENV-T022, ENV-T027, ENV-T034, ENV-T035, ENV-T036, ENV-T037, ENV-T038, ENV-T039, ENV-T040, ENV-T045.
- **Entregables:** Undo/redo en UI; aviso de symlink; ocultación; copia; protección al cerrar; diálogo de pegado y resolución; bienvenida.
- **Criterios de aceptación del sprint:** Undo/redo con foco correcto; symlink nunca reemplazado; cierre nunca descarta sin elección; pegado sin reemplazos implícitos. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** D-16 y D-17 decididas el 2027-01-18; acumulación de defectos de UI.
- **Demo esperada:** Crear, eliminar, deshacer; pegar con duplicado; cerrar con cambios.

#### S10 — Integración Quick Edit y catálogo

- **Fechas:** 2027-01-25 → 2027-01-29.
- **Fases cubiertas:** Fase C — Quick Edit, Fase D — Workspace y herramientas de entorno.
- **Capacidad:** 41 h brutas / 32.8 h netas · **planificado:** 31 h · holgura 1.8 h.
- **Objetivo:** Estabilizar Quick Edit (M3) e iniciar el Workspace con catálogo, proyectos y descubrimiento.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T005 Extender la CI con build de UI y pruebas GTK bajo display virtual | 4 | P1 | ENV-T003, ENV-T033 |
| 2 | ENV-T053 Integrar Quick Edit de extremo a extremo y corregir defectos | 6 | P1 | ENV-T042, ENV-T044, ENV-T046, ENV-T047, ENV-T048, ENV-T049, ENV-T051, ENV-T052, ENV-T043, ENV-T041, ENV-T038 |
| 3 | ENV-T055 Implementar el catálogo de proyectos sin secretos | 6 | P2 | ENV-T054, ENV-T004 |
| 4 | ENV-T071 Redactar el guion de pruebas manuales de UI y sistema | 4 | P1 | ENV-T053 |
| 5 | ENV-T056 Implementar la gestión de proyectos y carpetas de servicio | 6 | P2 | ENV-T055, ENV-T033 |
| 6 | ENV-T057 Implementar el descubrimiento de archivos dotenv en carpetas registradas | 5 | P2 | ENV-T055, ENV-T027 |

- **Dependencias de sprints anteriores:** ENV-T003, ENV-T004, ENV-T027, ENV-T033, ENV-T038, ENV-T041, ENV-T042, ENV-T043, ENV-T044, ENV-T046, ENV-T047, ENV-T048, ENV-T049, ENV-T051, ENV-T052, ENV-T054.
- **Entregables:** CI de UI; Quick Edit integrado; ProjectStore; guion manual; gestión de proyectos/servicios; descubrimiento.
- **Criterios de aceptación del sprint:** Situaciones A y D completas, defectos P0 = 0; catálogo sin valores; descubrimiento exacto con negativos. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** D-19, D-10 y D-29 decididas el 2027-01-25; buffer de integración (ENV-T053) puede quedarse corto.
- **Demo esperada:** Situaciones A y D en nativo; registrar una carpeta y listar sus dotenv. Hito M3.

### Fase D — Workspace y herramientas de entorno

#### S11 — Navegación del Workspace

- **Fechas:** 2027-02-01 → 2027-02-05.
- **Capacidad:** 41 h brutas / 32.8 h netas · **planificado:** 31 h · holgura 1.8 h.
- **Objetivo:** Navegar proyectos y archivos, cambiar de sesión sin pérdida y adaptar la ventana.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T059 Implementar la barra lateral del Workspace con AdwNavigationSplitView | 6 | P2 | ENV-T056, ENV-T057 |
| 2 | ENV-T058 Implementar el refresco del descubrimiento por monitor y acción Refrescar | 3 | P2 | ENV-T057 |
| 3 | ENV-T066 Implementar los avisos de claves duplicadas en el editor | 3 | P2 | ENV-T020, ENV-T036 |
| 4 | ENV-T060 Implementar la selección de archivo y la conmutación de sesiones | 5 | P2 | ENV-T059, ENV-T036, ENV-T035 |
| 5 | ENV-T061 Implementar estados vacíos y restauración de la última vista | 4 | P2 | ENV-T059, ENV-T052 |
| 6 | ENV-T074 Medir la línea base de rendimiento y la métrica principal | 3 | P1 | ENV-T053 |
| 7 | ENV-T062 Implementar la navegación adaptable en ventana estrecha | 4 | P1 | ENV-T059 |
| 8 | ENV-T072 Consolidar y verificar los atajos de teclado | 3 | P1 | ENV-T046, ENV-T047, ENV-T052, ENV-T045 |

- **Dependencias de sprints anteriores:** ENV-T020, ENV-T035, ENV-T036, ENV-T045, ENV-T046, ENV-T047, ENV-T052, ENV-T053, ENV-T056, ENV-T057.
- **Entregables:** Barra lateral; refresco; avisos de duplicados; conmutación de sesiones; estados vacíos; línea base de rendimiento; navegación adaptable; atajos consolidados.
- **Criterios de aceptación del sprint:** Cambiar de archivo conserva cambios; restauración tras reinicio; flujo completo a 360 px; atajos de PRD §7.4 verificados. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** Sesiones duplicadas por URI; rendimiento con archivos grandes (R-05).
- **Demo esperada:** Situación B: elegir proyecto, cambiar entre archivos modificados, ventana estrecha.

#### S12 — Herramientas de entorno, accesibilidad y Flatpak

- **Fechas:** 2027-02-08 → 2027-02-12.
- **Fases cubiertas:** Fase D — Workspace y herramientas de entorno, Fase E — Endurecimiento y distribución.
- **Capacidad:** 41 h brutas / 32.8 h netas · **planificado:** 29 h · holgura 3.8 h.
- **Objetivo:** Cerrar el Workspace (M4), auditar accesibilidad y construir el Flatpak.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T064 Implementar las etiquetas de entorno visibles y corregibles | 5 | P2 | ENV-T024, ENV-T059, ENV-T055 |
| 2 | ENV-T065 Implementar la comparación de claves con .env.example e inserción de faltantes | 6 | P2 | ENV-T023, ENV-T045, ENV-T060 |
| 3 | ENV-T063 Integrar Workspace de extremo a extremo y corregir defectos | 6 | P2 | ENV-T058, ENV-T060, ENV-T061, ENV-T062, ENV-T064, ENV-T065, ENV-T066 |
| 4 | ENV-T073 Auditar accesibilidad y HIG y corregir hallazgos | 6 | P1 | ENV-T062, ENV-T064, ENV-T065, ENV-T072 |
| 5 | ENV-T077 Crear el manifiesto Flatpak con dependencias Cargo offline | 6 | P0 | ENV-T054, ENV-T033 |

- **Dependencias de sprints anteriores:** ENV-T023, ENV-T024, ENV-T033, ENV-T045, ENV-T054, ENV-T055, ENV-T058, ENV-T059, ENV-T060, ENV-T061, ENV-T062, ENV-T066, ENV-T072.
- **Entregables:** Etiquetas; comparación con .env.example; Workspace integrado; auditoría A11y; manifiesto Flatpak.
- **Criterios de aceptación del sprint:** Situaciones B, C y E completas; todas las acciones esenciales con teclado; build Flatpak local. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** D-18 y D-07 (obligatoria) decididas el 2027-02-08.
- **Demo esperada:** Situación E; recorrido con Orca; Envy en sandbox. Hito M4.

### Fase E — Endurecimiento y distribución

#### S13 — Metadatos, i18n y validación Flatpak

- **Fechas:** 2027-02-15 → 2027-02-19.
- **Capacidad:** 41 h brutas / 32.8 h netas · **planificado:** 32 h · holgura 0.8 h.
- **Objetivo:** Completar metadatos, traducciones preparadas, icono y validar el flujo completo en Flatpak.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T078 Crear el desktop entry, los metadatos AppStream y el esquema de ajustes | 5 | P1 | ENV-T033 |
| 2 | ENV-T080 Preparar la internacionalización con gettext | 6 | P1 | ENV-T063, ENV-T053 |
| 3 | ENV-T079 Diseñar el icono de la aplicación y su variante simbólica | 5 | P1 | Ninguna |
| 4 | ENV-T081 Configurar la asociación de archivos y documentar la CLI en Flatpak | 5 | P1 | ENV-T077, ENV-T034, ENV-T078 |
| 5 | ENV-T082 Validar el flujo completo en Flatpak y la matriz de permisos bajo portal | 6 | P0 | ENV-T077, ENV-T061, ENV-T042, ENV-T060 |
| 6 | ENV-T086 Corregir los defectos detectados en la validación Flatpak | 5 | P0 | ENV-T082 |

- **Dependencias de sprints anteriores:** ENV-T033, ENV-T034, ENV-T042, ENV-T053, ENV-T060, ENV-T061, ENV-T063, ENV-T077.
- **Entregables:** Desktop/metainfo; gettext y .pot; icono; asociación de archivos; resultados TD §12; correcciones Flatpak.
- **Criterios de aceptación del sprint:** Validadores sin errores; .pot completo; cada paso de TD §12 con evidencia o limitación registrada. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** D-20 y D-21 decididas el 2027-02-15; limitaciones de portales (R-02, R-12).
- **Demo esperada:** Flujo de TD §12 en Flatpak con evidencia.

#### S14 — Endurecimiento, documentación y RC

- **Fechas:** 2027-02-22 → 2027-02-26.
- **Fases cubiertas:** Fase E — Endurecimiento y distribución, Fase F — Release candidate y v0.1.0.
- **Capacidad:** 41 h brutas / 32.8 h netas · **planificado:** 18 h · holgura 14.8 h.
- **Objetivo:** Completar la regresión nativa (M5) y generar el release candidate (M6).

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T075 Endurecer en nativo con regresión completa y corrección de defectos | 6 | P0 | ENV-T073, ENV-T063, ENV-T069, ENV-T067 |
| 2 | ENV-T083 Redactar README, añadir la licencia y documentar limitaciones | 5 | P1 | ENV-T082, ENV-T081 |
| 3 | ENV-T084 Verificar el cumplimiento de requisitos de Flathub | 4 | P1 | ENV-T077, ENV-T078, ENV-T079, ENV-T083 |
| 4 | ENV-T085 Preparar el release candidate v0.1.0-rc | 3 | P0 | ENV-T084, ENV-T083 |

- **Dependencias de sprints anteriores:** ENV-T063, ENV-T067, ENV-T069, ENV-T073, ENV-T077, ENV-T078, ENV-T079, ENV-T081, ENV-T082.
- **Entregables:** Informe de regresión; README, LICENSE, capturas; lint de Flathub; build RC.
- **Criterios de aceptación del sprint:** Defectos P0 = 0; auditoría de fugas con UI limpia; README reproducible; RC reproducible. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** D-08 y D-22 decididas el 2027-02-22; 14,8 h de holgura disponibles para defectos tardíos.
- **Demo esperada:** Informe de regresión e instalación del RC en limpio. Hitos M5 y M6.

### Fase F — Release candidate y v0.1.0

#### S15 — Regresión RC y release

- **Fechas:** 2027-03-01 → 2027-03-05.
- **Capacidad:** 41 h brutas / 32.8 h netas · **planificado:** 11 h · holgura 21.8 h.
- **Objetivo:** Evaluar go/no-go con una semana de margen sobre el RC y publicar v0.1.0 con aprobación.

| Orden | Tarea | h | Prio | Depende de |
|---|---|---|---|---|
| 1 | ENV-T076 Ejecutar la regresión del RC y preparar la evaluación go/no-go | 5 | P0 | ENV-T085, ENV-T075, ENV-T082, ENV-T086 |
| 2 | ENV-T088 Corregir defectos bloqueantes del RC | 4 | P0 | ENV-T085 |
| 3 | ENV-T087 Publicar v0.1.0 tras la aprobación go | 2 | P0 | ENV-T076, ENV-T088 |

- **Dependencias de sprints anteriores:** ENV-T075, ENV-T082, ENV-T085, ENV-T086.
- **Entregables:** Informe go/no-go; release publicada si go.
- **Criterios de aceptación del sprint:** Criterios de release con evidencia; aprobación explícita del usuario. Todas las tareas cumplen la DoD de tarea o se devuelven al backlog con motivo.
- **Riesgos/bloqueos:** No-go => ciclo adicional de corrección; 21,8 h de holgura absorben correcciones sin mover la fecha si son menores.
- **Demo esperada:** v0.1.0 instalada en limpio. Hito M7.

## 5. Reglas de replanificación

- Si una tarea supera su estimación en más de un 50 %, se detiene, se reestima y se informa antes de continuar.
- La holgura y la reserva absorben desvíos; si un sprint termina con trabajo no completado, la tarea pasa al siguiente sprint y se desplaza el resto **solo tras informar** del impacto en hitos.
- Un spike con resultado negativo (ENV-T025, ENV-T054) dispara una decisión del usuario antes de planificar las tareas dependientes.
- Cambios de alcance solo vía PRD aprobado; se registran en [DECISIONS.md](DECISIONS.md).
- Con 41 h/semana, si dos semanas seguidas se completa menos del 80 % de lo planificado, se replanifica con la capacidad real observada (R-17).

## 6. Historial del plan

| Versión | Fecha | Cambio |
|---|---|---|
| 1 | 2026-11-20 | Plan inicial con 15 h/semana supuestas: 38 sprints, v0.1.0 estimada 2027-08-13. |
| 2 | 2026-11-20 | D-02 resuelta: 41 h/semana brutas. Mismas 88 tareas / 417 h. 15 sprints, v0.1.0 estimada 2027-03-05. |
