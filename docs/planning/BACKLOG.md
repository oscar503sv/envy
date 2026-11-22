# Envy v0.1.0 — Product Backlog

> **Estado:** borrador de planificación pendiente de aprobación (generado 2026-11-20).  
> **Fuentes:** [`docs/PRD.md`](../PRD.md) v1.1 y [`docs/TECHNICAL_DESIGN.md`](../TECHNICAL_DESIGN.md) v1.1. Ante discrepancia de alcance prevalece el PRD (PRD, nota final).  
> **Documentos relacionados:** [ROADMAP](ROADMAP.md) · [SPRINT_PLAN](SPRINT_PLAN.md) · [MILESTONES](MILESTONES.md) · [RISKS](RISKS.md) · [DEFINITION_OF_DONE](DEFINITION_OF_DONE.md) · [DECISIONS](DECISIONS.md) · [TASK_TEMPLATE](TASK_TEMPLATE.md)

## 1. Convenciones

- **IDs de tarea:** `ENV-T###`. El PRD ya usa `ENV-001…ENV-020` para **requisitos**; reutilizar ese formato para tareas rompería la trazabilidad. El prefijo `ENV-T` está **propuesto** y pendiente de aprobación (ver [D-01](DECISIONS.md#d-01)). Los IDs son estables: no se renumeran; una tarea eliminada se marca `Descartada`.
- **Requisitos:** `ENV-0xx` (PRD §5), `US-0x` (PRD §6), `PRD §n` / `TD §n` para secciones, `D-xx` para decisiones de [DECISIONS.md](DECISIONS.md).
- **Prioridad:** misma semántica que el PRD §5 ([D-04](DECISIONS.md#d-04)): **P0** = seguridad o corrección sin la cual no se lanza; **P1** = experiencia esencial; **P2** = organización del Workspace. **Las tres forman parte del MVP**; la prioridad ordena la construcción. Las tareas habilitadoras (fundación, spikes, QA, empaquetado) heredan la prioridad más alta de lo que desbloquean.
- **Estimación:** horas de trabajo humano de una persona supervisando agentes de IA, incluyendo diseño de la solución, revisión del código generado, pruebas, integración y corrección. No se asume que la IA acierta a la primera.
- **Estado inicial:** todas las tareas están en `Backlog`. Flujo Kanban: `Backlog → Ready → In Progress → Review → QA → Done` (ver [DEFINITION_OF_DONE.md](DEFINITION_OF_DONE.md)).
- **Agentes:** roles previstos para ejecutar la tarea con [TASK_TEMPLATE.md](TASK_TEMPLATE.md): Explorador de código, Implementador, QA Engineer, Security Reviewer. Cuando figura *Security Reviewer*, su aprobación es obligatoria para pasar a `Done`.
- **Spikes:** investigación con timebox, hipótesis y criterio de salida; su código es desechable y no se integra en `main`.
- **Terminado:** cada tarea está terminada cuando cumple sus criterios de aceptación **y** la DoD de tarea ([DEFINITION_OF_DONE.md §2](DEFINITION_OF_DONE.md#2-definition-of-done--tarea)).

## 2. Resumen

- **Tareas:** 88 · **Épicas:** 9 · **Esfuerzo total estimado:** 417 h · **Sprints:** 15 (S01–S15).
- **Por prioridad:** P0: 45 tareas / 229 h · P1: 31 tareas / 131 h · P2: 12 tareas / 57 h.
- **Spikes:** 4 (ENV-T001, ENV-T025, ENV-T039, ENV-T054).
- **Tiempo explícito de integración/corrección de defectos:** 27 h en ENV-T053, ENV-T063, ENV-T075, ENV-T086, ENV-T088, además de la reserva del 20 % por sprint (ver [SPRINT_PLAN.md](SPRINT_PLAN.md)).

### 2.1 Épicas

Se mantienen las 9 épicas propuestas. Ajustes justificados: los **spikes** viven en la épica cuyo riesgo reducen; la lógica pura de duplicados, comparación de claves, etiquetas y heurística de secretos se ubica en *Editing Engine* porque es testeable sin GTK (ADR-002) y la reutilizan pegado y revisión de conflictos; *Quality Assurance* solo contiene verificación transversal (cada tarea de implementación incluye ya sus pruebas unitarias).

| Épica | Nombre | Alcance | Tareas | Horas |
|---|---|---|---|---|
| E1 | Project Foundation | Toolchain, esqueleto del crate, CI, política de errores/diagnóstico sin secretos. | 5 | 19 |
| E2 | Document Engine | Especificación de sintaxis, modelo de nodos con spans, parser, serializador, ValueCodec y fidelidad byte a byte. | 10 | 48 |
| E3 | Editing Engine | Comandos tipados (SetValue/Insert/Remove/Batch), historial, pegado, duplicados, heurística de secretos, comparación de claves e inferencia de etiquetas (lógica pura). | 9 | 36 |
| E4 | Filesystem & Safety | Puerto de almacenamiento, lectura/guardado condicionado con Gio, DocumentSession, monitor de cambios y revisión de conflictos. | 8 | 45 |
| E5 | Quick Edit | Aplicación GTK4/libadwaita, CLI/apertura, EditorView compartido y todas las interacciones de edición. | 21 | 98 |
| E6 | Workspace | Catálogo de proyectos sin secretos, servicios, descubrimiento, navegación adaptable y estados vacíos. | 10 | 53 |
| E7 | Environment Tools | Etiquetas de entorno, comparación con .env.example y avisos de duplicados en la UI. | 3 | 14 |
| E8 | Quality Assurance | Verificación transversal: matrices de guardado y conflictos, regresión de edición, fugas de secretos, accesibilidad, rendimiento, endurecimiento y RC. | 10 | 48 |
| E9 | Distribution & Release | Flatpak, metadatos AppStream/desktop, icono, i18n, asociación de archivos, README/licencia y publicación. | 12 | 56 |
| | **Total** | | **88** | **417** |

## 3. Matriz de trazabilidad (requisito → componente → entregables → pruebas)

| Requisito | Prio PRD | Componente (TD §4) | Tareas | Verificación |
|---|---|---|---|---|
| ENV-001 Parser sin pérdida | P0 | domain/document, domain/syntax | ENV-T006, ENV-T007, ENV-T008, ENV-T009, ENV-T010, ENV-T011, ENV-T012, ENV-T013 | tests/fidelity.rs, proptest (T013), T075 |
| ENV-002 Edición de asignaciones | P0 | domain/syntax (ValueCodec), domain/edit, ui/assignment_row | ENV-T010, ENV-T011, ENV-T014, ENV-T016, ENV-T037, ENV-T038 | T068, guion manual |
| ENV-003 Sintaxis desconocida | P0 | domain/syntax (Raw), ui/editor | ENV-T006, ENV-T010, ENV-T011, ENV-T036, ENV-T041 | fixtures unknown-syntax/unclosed-quotes, T068 |
| ENV-004 Guardado protegido | P0 | services/DocumentStorage, infrastructure/gio_storage, ui/window | ENV-T025, ENV-T026, ENV-T027, ENV-T028, ENV-T030, ENV-T040, ENV-T043 | T067, T082 |
| ENV-005 Monitoreo de archivo | P0 | services/DocumentSession, infrastructure/file_monitor, ui/conflict_view | ENV-T029, ENV-T031, ENV-T032, ENV-T042 | T069, guion manual |
| ENV-006 Quick Edit | P1 | application, cli, ui/window | ENV-T033, ENV-T034, ENV-T035, ENV-T052, ENV-T081 | guion manual, T053 |
| ENV-007 Buscar | P1 | ui/editor | ENV-T047 | test de filtro, guion manual |
| ENV-008 Crear/eliminar | P1 | domain/edit, ui/editor | ENV-T017, ENV-T018, ENV-T045 | T068 |
| ENV-009 Undo/redo | P1 | domain/history, ui/window | ENV-T019, ENV-T039, ENV-T046 | T068, guion manual (D-13) |
| ENV-010 Paste assignments | P1 | domain/edit (PastePlan), ui/paste_dialog | ENV-T020, ENV-T021, ENV-T050, ENV-T051 | T068 |
| ENV-011 Copiar | P1 | ui/assignment_row, ui/window | ENV-T049 | test portapapeles = serialize |
| ENV-012 Secretos | P1 | domain/secrets, ui/assignment_row | ENV-T022, ENV-T048 | tests tabulares, Orca |
| ENV-013 Estado de cambios | P1 | services/DocumentSession, ui/window | ENV-T019, ENV-T029, ENV-T041 | tests de transiciones, guion manual |
| ENV-014 Workspace | P2 | domain/project, infrastructure/project_store, ui/workspace | ENV-T055, ENV-T056, ENV-T059, ENV-T060, ENV-T061 | tests de catálogo, T063, T082 |
| ENV-015 Servicios | P2 | services/project_service, ui/workspace | ENV-T055, ENV-T056 | T063 |
| ENV-016 Descubrimiento | P2 | services/environment_service | ENV-T057, ENV-T058 | tests con tempdir |
| ENV-017 Etiquetas de entorno | P2 | domain/project, ui/workspace | ENV-T024, ENV-T064 | tests tabulares, contraste |
| ENV-018 Comparación de claves | P2 | domain/compare, ui/editor | ENV-T023, ENV-T065 | tests unitarios, Situación E |
| ENV-019 Duplicados | P2 | domain/compare, ui/assignment_row | ENV-T020, ENV-T066 | fixture duplicate-keys |
| ENV-020 Atajos y accesibilidad | P1 | ui/* | ENV-T062, ENV-T072, ENV-T073 | auditoría A11y, guion manual |
| PRD §8 Seguridad y privacidad | P0 | transversal | ENV-T004, ENV-T015, ENV-T043, ENV-T054, ENV-T070, ENV-T077 | T070, T075 (con UI) |
| PRD §9/§12 Flatpak y portales | P0 | packaging/flatpak | ENV-T054, ENV-T077, ENV-T081, ENV-T082, ENV-T086 | T082 |
| PRD §10 Métricas | P1 | transversal | ENV-T012, ENV-T074 | fidelidad 100 %, línea base |
| PRD §10 Cero pérdidas (cierre) | P0 | ui/window, application | ENV-T044 | guion manual (D-16) |
| PRD §11 hito 4 / §14 DoD release | P1 | data/, docs, po/ | ENV-T078, ENV-T079, ENV-T080, ENV-T083, ENV-T084, ENV-T085, ENV-T087 | validadores, T076 |

Ningún requisito del PRD §5 queda sin tarea. No se incluyen funciones de PRD §2.2 (no objetivos).

## 4. Índice de tareas

| ID | Título | Épica | Prio | h | Sprint | Depende de |
|---|---|---|---|---|---|---|
| [ENV-T001](#env-t001) | Spike: validar toolchain y compatibilidad gtk4-rs/libadwaita/runtime GNOME | E1 | P0 | 4 | S01 | Ninguna |
| [ENV-T002](#env-t002) | Crear el esqueleto del crate con módulos por capas | E1 | P0 | 3 | S01 | Ninguna |
| [ENV-T003](#env-t003) | Configurar CI del núcleo con fmt, clippy y tests | E1 | P0 | 4 | S02 | ENV-T002 |
| [ENV-T004](#env-t004) | Definir tipos de error y política de diagnóstico sin secretos | E1 | P0 | 4 | S01 | ENV-T002 |
| [ENV-T005](#env-t005) | Extender la CI con build de UI y pruebas GTK bajo display virtual | E1 | P1 | 4 | S10 | ENV-T003, ENV-T033 |
| [ENV-T006](#env-t006) | Redactar la especificación de sintaxis dotenv v0.1 | E2 | P0 | 5 | S01 | Ninguna |
| [ENV-T007](#env-t007) | Crear el corpus de fixtures de fidelidad | E2 | P0 | 4 | S01 | ENV-T006 |
| [ENV-T008](#env-t008) | Implementar el modelo EnvDocument/EnvNode con spans absolutos | E2 | P0 | 4 | S01 | ENV-T002, ENV-T006, ENV-T004 |
| [ENV-T009](#env-t009) | Implementar segmentación de líneas, BOM y compuerta UTF-8 | E2 | P0 | 5 | S01 | ENV-T008 |
| [ENV-T010](#env-t010) | Implementar el reconocimiento de asignaciones de una línea | E2 | P0 | 7 | S02 | ENV-T009, ENV-T007 |
| [ENV-T011](#env-t011) | Implementar valores multilínea entre comillas | E2 | P0 | 6 | S02 | ENV-T010 |
| [ENV-T012](#env-t012) | Implementar el serializador puro y el arnés de fidelidad byte a byte | E2 | P0 | 4 | S02 | ENV-T010, ENV-T011, ENV-T007 |
| [ENV-T013](#env-t013) | Añadir pruebas de propiedades del roundtrip parser/serializador | E2 | P0 | 4 | S02 | ENV-T012 |
| [ENV-T014](#env-t014) | Implementar ValueCodec por estilo de comillas | E2 | P0 | 7 | S03 | ENV-T006, ENV-T011 |
| [ENV-T015](#env-t015) | Crear el ejemplo interno roundtrip de verificación de fidelidad | E2 | P1 | 2 | S03 | ENV-T012, ENV-T004 |
| [ENV-T016](#env-t016) | Implementar el comando SetValue con reemplazo mínimo | E3 | P0 | 6 | S03 | ENV-T014, ENV-T012 |
| [ENV-T017](#env-t017) | Implementar el comando InsertAssignment al final del documento | E3 | P1 | 5 | S04 | ENV-T012 |
| [ENV-T018](#env-t018) | Implementar el comando RemoveAssignment de una ocurrencia | E3 | P1 | 3 | S04 | ENV-T012 |
| [ENV-T019](#env-t019) | Implementar EditHistory con lotes transaccionales y cálculo de dirty | E3 | P0 | 6 | S04 | ENV-T016, ENV-T017, ENV-T018 |
| [ENV-T020](#env-t020) | Implementar el diagnóstico de claves duplicadas | E3 | P1 | 2 | S03 | ENV-T010 |
| [ENV-T021](#env-t021) | Implementar el núcleo de pegado de asignaciones (PastePlan) | E3 | P1 | 7 | S06 | ENV-T011, ENV-T019, ENV-T020 |
| [ENV-T022](#env-t022) | Implementar la heurística de secretos por nombre de clave | E3 | P1 | 2 | S03 | ENV-T008 |
| [ENV-T023](#env-t023) | Implementar la comparación de conjuntos de claves entre documentos | E3 | P0 | 3 | S03 | ENV-T010 |
| [ENV-T024](#env-t024) | Implementar la inferencia de etiqueta de entorno por nombre de archivo | E3 | P2 | 2 | S01 | ENV-T002 |
| [ENV-T025](#env-t025) | Spike: semántica real de guardado condicionado con Gio en sistema local | E4 | P0 | 6 | S02 | ENV-T001, ENV-T002 |
| [ENV-T026](#env-t026) | Definir el puerto DocumentStorage y un almacenamiento falso para pruebas | E4 | P0 | 4 | S04 | ENV-T008, ENV-T004 |
| [ENV-T027](#env-t027) | Implementar la lectura segura de archivos en GioStorage | E4 | P0 | 6 | S04 | ENV-T026, ENV-T025 |
| [ENV-T028](#env-t028) | Implementar replace_if_unchanged con verificación posterior | E4 | P0 | 7 | S05 | ENV-T027, ENV-T025 |
| [ENV-T029](#env-t029) | Implementar DocumentSession y la máquina de estados del documento | E4 | P0 | 7 | S04 | ENV-T019, ENV-T026 |
| [ENV-T030](#env-t030) | Implementar la orquestación de Guardar y Guardar como | E4 | P0 | 5 | S05 | ENV-T029, ENV-T028 |
| [ENV-T031](#env-t031) | Implementar el adaptador FileMonitor con coalescencia y verificación por hash | E4 | P0 | 6 | S05 | ENV-T029, ENV-T027 |
| [ENV-T032](#env-t032) | Implementar el resumen de revisión de conflicto sin valores | E4 | P0 | 4 | S06 | ENV-T029, ENV-T023 |
| [ENV-T033](#env-t033) | Inicializar la aplicación GTK4/libadwaita y la ventana principal | E5 | P0 | 6 | S07 | ENV-T001, ENV-T002, ENV-T004 |
| [ENV-T034](#env-t034) | Implementar la CLI y el manejo de apertura de archivos | E5 | P1 | 5 | S07 | ENV-T033, ENV-T027 |
| [ENV-T035](#env-t035) | Implementar el registro de sesiones y ventanas por documento | E5 | P1 | 4 | S07 | ENV-T034, ENV-T029 |
| [ENV-T036](#env-t036) | Implementar EditorView con filas por tipo de nodo | E5 | P0 | 7 | S07 | ENV-T033, ENV-T029 |
| [ENV-T037](#env-t037) | Implementar la edición de valores de una línea con conversión explícita | E5 | P0 | 6 | S08 | ENV-T036, ENV-T039, ENV-T016, ENV-T014 |
| [ENV-T038](#env-t038) | Implementar la fila expandible para valores multilínea | E5 | P0 | 5 | S08 | ENV-T037 |
| [ENV-T039](#env-t039) | Spike: política única de undo y granularidad de confirmación de edición | E5 | P1 | 4 | S07 | ENV-T036 |
| [ENV-T040](#env-t040) | Implementar las acciones Guardar y Guardar como en la UI | E5 | P0 | 5 | S08 | ENV-T036, ENV-T030 |
| [ENV-T041](#env-t041) | Implementar indicadores de estado y modos de solo lectura | E5 | P1 | 4 | S07 | ENV-T036, ENV-T029 |
| [ENV-T042](#env-t042) | Implementar el banner de conflicto y sus flujos de resolución | E5 | P0 | 6 | S08 | ENV-T040, ENV-T031, ENV-T032 |
| [ENV-T043](#env-t043) | Implementar el aviso de enlace simbólico y la selección explícita del destino | E5 | P0 | 3 | S09 | ENV-T034, ENV-T027 |
| [ENV-T044](#env-t044) | Implementar la protección al cerrar con cambios sin guardar | E5 | P0 | 4 | S09 | ENV-T040, ENV-T035 |
| [ENV-T045](#env-t045) | Implementar agregar y eliminar asignaciones en la UI | E5 | P1 | 5 | S08 | ENV-T036, ENV-T017, ENV-T018 |
| [ENV-T046](#env-t046) | Integrar undo/redo del historial en la UI | E5 | P1 | 4 | S09 | ENV-T039, ENV-T019, ENV-T045, ENV-T037 |
| [ENV-T047](#env-t047) | Implementar la búsqueda de variables por clave | E5 | P1 | 4 | S08 | ENV-T036 |
| [ENV-T048](#env-t048) | Implementar la ocultación visual de secretos con revelado por fila | E5 | P1 | 5 | S09 | ENV-T036, ENV-T022, ENV-T038 |
| [ENV-T049](#env-t049) | Implementar copiar valor y copiar documento | E5 | P1 | 3 | S09 | ENV-T036, ENV-T014 |
| [ENV-T050](#env-t050) | Implementar el diálogo de pegado con previsualización | E5 | P1 | 5 | S09 | ENV-T021, ENV-T046 |
| [ENV-T051](#env-t051) | Implementar la resolución de duplicados y líneas inválidas en el pegado | E5 | P1 | 4 | S09 | ENV-T050 |
| [ENV-T052](#env-t052) | Implementar la pantalla de bienvenida y la acción Abrir archivo | E5 | P1 | 3 | S09 | ENV-T034 |
| [ENV-T053](#env-t053) | Integrar Quick Edit de extremo a extremo y corregir defectos | E5 | P1 | 6 | S10 | ENV-T042, ENV-T044, ENV-T046, ENV-T047, ENV-T048, ENV-T049, ENV-T051, ENV-T052, ENV-T043, ENV-T041, ENV-T038 |
| [ENV-T054](#env-t054) | Spike: acceso a carpetas, persistencia de permisos y CLI en Flatpak | E6 | P0 | 8 | S03 | ENV-T001, ENV-T025 |
| [ENV-T055](#env-t055) | Implementar el catálogo de proyectos sin secretos | E6 | P2 | 6 | S10 | ENV-T054, ENV-T004 |
| [ENV-T056](#env-t056) | Implementar la gestión de proyectos y carpetas de servicio | E6 | P2 | 6 | S10 | ENV-T055, ENV-T033 |
| [ENV-T057](#env-t057) | Implementar el descubrimiento de archivos dotenv en carpetas registradas | E6 | P2 | 5 | S10 | ENV-T055, ENV-T027 |
| [ENV-T058](#env-t058) | Implementar el refresco del descubrimiento por monitor y acción Refrescar | E6 | P2 | 3 | S11 | ENV-T057 |
| [ENV-T059](#env-t059) | Implementar la barra lateral del Workspace con AdwNavigationSplitView | E6 | P2 | 6 | S11 | ENV-T056, ENV-T057 |
| [ENV-T060](#env-t060) | Implementar la selección de archivo y la conmutación de sesiones | E6 | P2 | 5 | S11 | ENV-T059, ENV-T036, ENV-T035 |
| [ENV-T061](#env-t061) | Implementar estados vacíos y restauración de la última vista | E6 | P2 | 4 | S11 | ENV-T059, ENV-T052 |
| [ENV-T062](#env-t062) | Implementar la navegación adaptable en ventana estrecha | E6 | P1 | 4 | S11 | ENV-T059 |
| [ENV-T063](#env-t063) | Integrar Workspace de extremo a extremo y corregir defectos | E6 | P2 | 6 | S12 | ENV-T058, ENV-T060, ENV-T061, ENV-T062, ENV-T064, ENV-T065, ENV-T066 |
| [ENV-T064](#env-t064) | Implementar las etiquetas de entorno visibles y corregibles | E7 | P2 | 5 | S12 | ENV-T024, ENV-T059, ENV-T055 |
| [ENV-T065](#env-t065) | Implementar la comparación de claves con .env.example e inserción de faltantes | E7 | P2 | 6 | S12 | ENV-T023, ENV-T045, ENV-T060 |
| [ENV-T066](#env-t066) | Implementar los avisos de claves duplicadas en el editor | E7 | P2 | 3 | S11 | ENV-T020, ENV-T036 |
| [ENV-T067](#env-t067) | Crear la matriz de pruebas de integración de guardado seguro | E8 | P0 | 6 | S05 | ENV-T028 |
| [ENV-T068](#env-t068) | Crear la suite de regresión de edición | E8 | P0 | 5 | S06 | ENV-T019, ENV-T021 |
| [ENV-T069](#env-t069) | Crear las pruebas de integración de conflictos y monitorización | E8 | P0 | 6 | S06 | ENV-T030, ENV-T031 |
| [ENV-T070](#env-t070) | Auditar y probar la ausencia de secretos en logs y errores del núcleo | E8 | P0 | 4 | S06 | ENV-T030, ENV-T021 |
| [ENV-T071](#env-t071) | Redactar el guion de pruebas manuales de UI y sistema | E8 | P1 | 4 | S10 | ENV-T053 |
| [ENV-T072](#env-t072) | Consolidar y verificar los atajos de teclado | E8 | P1 | 3 | S11 | ENV-T046, ENV-T047, ENV-T052, ENV-T045 |
| [ENV-T073](#env-t073) | Auditar accesibilidad y HIG y corregir hallazgos | E8 | P1 | 6 | S12 | ENV-T062, ENV-T064, ENV-T065, ENV-T072 |
| [ENV-T074](#env-t074) | Medir la línea base de rendimiento y la métrica principal | E8 | P1 | 3 | S11 | ENV-T053 |
| [ENV-T075](#env-t075) | Endurecer en nativo con regresión completa y corrección de defectos | E8 | P0 | 6 | S14 | ENV-T073, ENV-T063, ENV-T069, ENV-T067 |
| [ENV-T076](#env-t076) | Ejecutar la regresión del RC y preparar la evaluación go/no-go | E8 | P0 | 5 | S15 | ENV-T085, ENV-T075, ENV-T082, ENV-T086 |
| [ENV-T077](#env-t077) | Crear el manifiesto Flatpak con dependencias Cargo offline | E9 | P0 | 6 | S12 | ENV-T054, ENV-T033 |
| [ENV-T078](#env-t078) | Crear el desktop entry, los metadatos AppStream y el esquema de ajustes | E9 | P1 | 5 | S13 | ENV-T033 |
| [ENV-T079](#env-t079) | Diseñar el icono de la aplicación y su variante simbólica | E9 | P1 | 5 | S13 | Ninguna |
| [ENV-T080](#env-t080) | Preparar la internacionalización con gettext | E9 | P1 | 6 | S13 | ENV-T063, ENV-T053 |
| [ENV-T081](#env-t081) | Configurar la asociación de archivos y documentar la CLI en Flatpak | E9 | P1 | 5 | S13 | ENV-T077, ENV-T034, ENV-T078 |
| [ENV-T082](#env-t082) | Validar el flujo completo en Flatpak y la matriz de permisos bajo portal | E9 | P0 | 6 | S13 | ENV-T077, ENV-T061, ENV-T042, ENV-T060 |
| [ENV-T083](#env-t083) | Redactar README, añadir la licencia y documentar limitaciones | E9 | P1 | 5 | S14 | ENV-T082, ENV-T081 |
| [ENV-T084](#env-t084) | Verificar el cumplimiento de requisitos de Flathub | E9 | P1 | 4 | S14 | ENV-T077, ENV-T078, ENV-T079, ENV-T083 |
| [ENV-T085](#env-t085) | Preparar el release candidate v0.1.0-rc | E9 | P0 | 3 | S14 | ENV-T084, ENV-T083 |
| [ENV-T086](#env-t086) | Corregir los defectos detectados en la validación Flatpak | E9 | P0 | 5 | S13 | ENV-T082 |
| [ENV-T087](#env-t087) | Publicar v0.1.0 tras la aprobación go | E9 | P0 | 2 | S15 | ENV-T076, ENV-T088 |
| [ENV-T088](#env-t088) | Corregir defectos bloqueantes del RC | E9 | P0 | 4 | S15 | ENV-T085 |
| | **Total** | | | **417** | | |

## 5. Detalle de tareas por épica

### E1 — Project Foundation

<a id="env-t001"></a>
#### ENV-T001 — Spike: validar toolchain y compatibilidad gtk4-rs/libadwaita/runtime GNOME

| Campo | Valor |
|---|---|
| Épica | E1 Project Foundation |
| Origen (requisito/decisión) | TD §3, TD §16; PRD §13 (runtime mínimo); decisiones D-05, D-06, D-09, D-32 |
| Prioridad | P0 |
| Estimación | 4 h |
| Depende de | Ninguna |
| Bloquea a | ENV-T025, ENV-T033, ENV-T054 |
| Sprint propuesto | S01 (2026-11-23 → 2026-11-27) |
| Estado | Backlog |
| Agentes | Explorador, Implementador |
| Entregables / archivos | Propuesta de versiones y sistema de build anotada en docs/planning/DECISIONS.md (D-05, D-06, D-09, D-32); prototipo desechable fuera de main. |

**Descripción.** Spike con timebox de 4 h. Hipótesis: existe una combinación Rust estable (edición 2024) + gtk4-rs + libadwaita-rs + runtime GNOME de Flathub que compila una ventana Adw mínima en nativo y con el SDK Flatpak, con una versión de libadwaita que incluye los widgets previstos (AdwNavigationSplitView, AdwToolbarView, AdwAlertDialog). El código del spike es desechable y no se integra en main.

**Criterios de aceptación**
- [ ] Tabla propuesta: versión de Rust/edición, gtk4-rs, libadwaita-rs, libadwaita mínima y runtime GNOME, con fuente de cada dato.
- [ ] Ventana AdwApplicationWindow mínima compilada y ejecutada en nativo (evidencia: comando y salida).
- [ ] Recomendación Cargo+build.rs vs Meson con trade-offs; criterio de salida: decisión propuesta al usuario.

**Pruebas:** Build y ejecución manual del prototipo nativo; build con SDK Flatpak si está disponible.

**Riesgos / seguridad:** Si el SDK Flatpak no está instalado, instalarlo requiere autorización del usuario. Resultado negativo => revisar libadwaita mínima (D-05).

<a id="env-t002"></a>
#### ENV-T002 — Crear el esqueleto del crate con módulos por capas

| Campo | Valor |
|---|---|
| Épica | E1 Project Foundation |
| Origen (requisito/decisión) | TD ADR-001, ADR-002, §4 |
| Prioridad | P0 |
| Estimación | 3 h |
| Depende de | Ninguna |
| Bloquea a | ENV-T003, ENV-T004, ENV-T008, ENV-T024, ENV-T025, ENV-T033 |
| Sprint propuesto | S01 (2026-11-23 → 2026-11-27) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | Cargo.toml, Cargo.lock, src/main.rs, src/{domain,services,infrastructure,ui}/mod.rs, rustfmt.toml, .gitignore |

**Descripción.** Crate único con módulos domain/, services/, infrastructure/, ui/ (vacíos o con stubs), Cargo.lock versionado, configuración rustfmt/clippy y .gitignore. Sin dependencias GTK todavía: el núcleo debe compilar y testearse sin GTK.

**Criterios de aceptación**
- [ ] cargo build, cargo test, cargo fmt --check y cargo clippy -- -D warnings en verde.
- [ ] cargo tree muestra que domain/ no arrastra gtk4/libadwaita.
- [ ] Estructura coincide con TD §4 o la desviación queda justificada.

**Pruebas:** Ejecución local de build/test/fmt/clippy (evidencia en la tarea).

<a id="env-t003"></a>
#### ENV-T003 — Configurar CI del núcleo con fmt, clippy y tests

| Campo | Valor |
|---|---|
| Épica | E1 Project Foundation |
| Origen (requisito/decisión) | TD §14.4; decisión D-23 (hosting CI) |
| Prioridad | P0 |
| Estimación | 4 h |
| Depende de | ENV-T002 |
| Bloquea a | ENV-T005 |
| Sprint propuesto | S02 (2026-11-30 → 2026-12-04) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | Workflow de CI (ruta según D-23, p. ej. .github/workflows/ci.yml) |

**Descripción.** Pipeline en runner Linux que ejecuta cargo fmt --check, cargo clippy -- -D warnings y cargo test sobre el núcleo sin GTK, con caché de dependencias.

**Criterios de aceptación**
- [ ] El pipeline corre en cada push/PR y queda en verde sobre main.
- [ ] Un warning de clippy introducido a propósito en una rama hace fallar el pipeline (evidencia).
- [ ] Los tiempos de ejecución quedan anotados para vigilar su crecimiento.

**Pruebas:** Ejecución real del pipeline en una rama de prueba.

**Heredado de la revisión de seguridad de ENV-T004 (2026-11-21):** añadir `cargo audit` o `cargo deny` y compilar con `--locked`.

**Riesgos / seguridad:** Hosting de CI no decidido (D-23); se asume GitHub Actions de forma provisional.

<a id="env-t004"></a>
#### ENV-T004 — Definir tipos de error y política de diagnóstico sin secretos

| Campo | Valor |
|---|---|
| Épica | E1 Project Foundation |
| Origen (requisito/decisión) | PRD §8.2; TD §13, ADR-007 |
| Prioridad | P0 |
| Estimación | 4 h |
| Depende de | ENV-T002 |
| Bloquea a | ENV-T008, ENV-T015, ENV-T026, ENV-T033, ENV-T055 |
| Sprint propuesto | S01 (2026-11-23 → 2026-11-27) |
| Estado | Backlog |
| Agentes | Implementador, QA, Security Reviewer |
| Entregables / archivos | src/domain/error.rs (o equivalente por capa), módulo de redacción, documentación de módulo |

**Descripción.** Errores de dominio con thiserror que nunca transportan valores ni contenido de archivo (solo línea, tipo de nodo, URI visible). Envoltorio `Redacted<T>` o Debug manual para tipos con valores. Reglas de uso de tracing documentadas en el módulo (qué se puede registrar). Política de panic sin contenido.

**Criterios de aceptación**
- [ ] format!("{:?}") y Display de errores y tipos con valores no contienen el valor centinela de prueba.
- [ ] La política lista explícitamente qué campos se pueden registrar y cuáles no.
- [ ] Ningún tipo con valores deriva Debug automáticamente sin redacción.

**Pruebas:** Tests unitarios con valor centinela sobre Debug/Display de errores y tipos de dominio.

**Riesgos / seguridad:** Base de la prevención de fugas; revisión de seguridad obligatoria.

<a id="env-t005"></a>
#### ENV-T005 — Extender la CI con build de UI y pruebas GTK bajo display virtual

| Campo | Valor |
|---|---|
| Épica | E1 Project Foundation |
| Origen (requisito/decisión) | TD §14.4 |
| Prioridad | P1 |
| Estimación | 4 h |
| Depende de | ENV-T003, ENV-T033 |
| Bloquea a | Ninguna |
| Sprint propuesto | S10 (2027-01-25 → 2027-01-29) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | Workflow de CI |

**Descripción.** Job adicional con dependencias GTK4/libadwaita que compila la UI y ejecuta las pruebas que requieren display (xvfb o compositor headless). Las pruebas GTK sin display se reportan como omitidas, nunca como aprobadas.

**Criterios de aceptación**
- [ ] La UI compila en CI con las versiones fijadas en Cargo.lock.
- [ ] Las pruebas GTK se ejecutan con display virtual y su resultado es visible.
- [ ] Sin display, el job marca las pruebas GTK como skipped de forma explícita.

**Pruebas:** Ejecución del job en rama de prueba, incluyendo un caso que falla a propósito.

### E2 — Document Engine

<a id="env-t006"></a>
#### ENV-T006 — Redactar la especificación de sintaxis dotenv v0.1

| Campo | Valor |
|---|---|
| Épica | E2 Document Engine |
| Origen (requisito/decisión) | ENV-001, ENV-002, ENV-003; PRD US-02, §13; TD §5.3; decisión D-11 |
| Prioridad | P0 |
| Estimación | 5 h |
| Depende de | Ninguna |
| Bloquea a | ENV-T007, ENV-T008, ENV-T014 |
| Sprint propuesto | S01 (2026-11-23 → 2026-11-27) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | docs/SYNTAX.md (ruta propuesta, nuevo documento de implementación) |

**Descripción.** Documento normativo del subconjunto soportado: regex de clave, espacios alrededor de =, prefijo export, fin de valor sin comillas, comentario final inequívoco, reglas de comillas simples/dobles y escapes (base del ValueCodec), cierre de multilínea, comillas sin cerrar, BOM, CR/LF/CRLF, valor vacío, casos Raw, estilo de salto de línea para contenido insertado y regla de newline final. Incluye lista de preguntas abiertas para aprobación del usuario.

**Criterios de aceptación**
- [ ] Cada fila de TD §5.3 tiene una regla explícita y un fixture asociado.
- [ ] Ninguna regla contradice PRD US-02; los casos dudosos se resuelven hacia Raw.
- [ ] Las preguntas abiertas quedan listadas en DECISIONS.md (D-11) para aprobación.

**Pruebas:** Revisión cruzada contra PRD/TD por QA (no hay código).

**Riesgos / seguridad:** Si la aprobación de D-11 se retrasa, T010/T011/T014 se basan en la propuesta y pueden requerir retrabajo.

<a id="env-t007"></a>
#### ENV-T007 — Crear el corpus de fixtures de fidelidad

| Campo | Valor |
|---|---|
| Épica | E2 Document Engine |
| Origen (requisito/decisión) | ENV-001; PRD §10 (100 % fixtures); TD §14.1 |
| Prioridad | P0 |
| Estimación | 4 h |
| Depende de | ENV-T006 |
| Bloquea a | ENV-T010, ENV-T012 |
| Sprint propuesto | S01 (2026-11-23 → 2026-11-27) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | tests/fixtures/*, tests/fixtures/expected/*, .gitattributes |

**Descripción.** Los 16 fixtures de TD §14.1 más los casos de la especificación (solo CR, variantes de export, comilla sin cerrar en medio, # dentro de comillas, KEY=$OTHER literal, claves con punto/guion como Raw). Archivos binario-exactos protegidos con .gitattributes (-text) para evitar normalización de EOL. Valores secretos ficticios con un centinela reconocible para las pruebas de fuga. Descriptor esperado de tipos de nodo por fixture.

**Criterios de aceptación**
- [ ] Git no altera bytes de los fixtures (verificado con hexdump antes/después de checkout).
- [ ] Cada fixture está referenciado en docs/SYNTAX.md y tiene descriptor de nodos esperado.
- [ ] Los valores secretos de fixtures usan el centinela acordado y no son secretos reales.

**Pruebas:** Verificación de bytes con hexdump/sha256 tras clonar en limpio.

<a id="env-t008"></a>
#### ENV-T008 — Implementar el modelo EnvDocument/EnvNode con spans absolutos

| Campo | Valor |
|---|---|
| Épica | E2 Document Engine |
| Origen (requisito/decisión) | ENV-001; TD ADR-003, §5.1, §5.2 (invariantes 1, 3, 4) |
| Prioridad | P0 |
| Estimación | 4 h |
| Depende de | ENV-T002, ENV-T006, ENV-T004 |
| Bloquea a | ENV-T009, ENV-T022, ENV-T026 |
| Sprint propuesto | S01 (2026-11-23 → 2026-11-27) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/document.rs, src/domain/syntax.rs |

**Descripción.** Tipos EnvDocument, EnvNode, NodeKind (Assignment/Comment/Blank/Raw), AssignmentSyntax, QuoteStyle e IDs de ocurrencia estables. Semántica de spans absoluta, origen Source(span) o Synthetic para nodos insertados, y función check_invariants usada en tests. Tipos con valores con Debug redactado (T004).

**Criterios de aceptación**
- [ ] Las invariantes 1, 3 y 4 de TD §5.2 se comprueban con check_invariants.
- [ ] Los nodos insertados no tienen span original y se distinguen por tipo.
- [ ] Debug de documentos y nodos no imprime valores.

**Pruebas:** Tests unitarios de invariantes y de Debug redactado.

**Heredado de la revisión de seguridad de ENV-T004 (2026-11-21):** cada tipo con valores (nodo, documento) lleva un test que mete `ENVY_TEST_SENTINEL_9F2C` en clave y valor y comprueba `{:?}` y `{:#?}`. Valorar `clippy::indexing_slicing` en el parser.

<a id="env-t009"></a>
#### ENV-T009 — Implementar segmentación de líneas, BOM y compuerta UTF-8

| Campo | Valor |
|---|---|
| Épica | E2 Document Engine |
| Origen (requisito/decisión) | ENV-001; PRD US-02 (BOM, LF/CRLF/CR/mixtos, UTF-8 no válido) |
| Prioridad | P0 |
| Estimación | 5 h |
| Depende de | ENV-T008 |
| Bloquea a | ENV-T010 |
| Sprint propuesto | S01 (2026-11-23 → 2026-11-27) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/syntax.rs (lexer de líneas) |

**Descripción.** Divide el flujo en líneas físicas con su terminador exacto (LF, CRLF, CR, mixtos, sin newline final), conserva el BOM sin normalizarlo y rechaza la edición estructurada de UTF-8 no válido con un error que indica posición, sin contenido.

**Criterios de aceptación**
- [ ] La concatenación de segmentos reproduce la entrada exacta para cualquier secuencia de bytes.
- [ ] utf8-bom.env conserva el BOM; mixed-newlines.env conserva cada terminador.
- [ ] invalid-utf8.bin devuelve error de edición estructurada sin modificar ni copiar bytes al mensaje.

**Pruebas:** Tests unitarios por tipo de terminador, BOM, archivo vacío y UTF-8 inválido.

<a id="env-t010"></a>
#### ENV-T010 — Implementar el reconocimiento de asignaciones de una línea

| Campo | Valor |
|---|---|
| Épica | E2 Document Engine |
| Origen (requisito/decisión) | ENV-001, ENV-002, ENV-003; TD §5.3 |
| Prioridad | P0 |
| Estimación | 7 h — incertidumbre media (comentarios finales ambiguos) |
| Depende de | ENV-T009, ENV-T007 |
| Bloquea a | ENV-T011, ENV-T012, ENV-T020, ENV-T023 |
| Sprint propuesto | S02 (2026-11-30 → 2026-12-04) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/syntax.rs |

**Descripción.** Reconoce KEY=value, espacios alrededor de =, export, valores sin comillas, simples, dobles, vacíos y comentario final inequívoco; comentarios y líneas vacías; todo lo demás como Raw. value_span exacto, sin comillas ni sufijo. $VAR, ${VAR} y $(cmd) son texto literal.

**Criterios de aceptación**
- [ ] La secuencia de tipos de nodo coincide con el descriptor esperado de cada fixture de una línea.
- [ ] value_span excluye comillas y comentario final; el sufijo se conserva.
- [ ] Claves fuera de ^[A-Za-z_][A-Za-z0-9_]*$ producen Raw; nada se expande ni ejecuta.

**Pruebas:** Tests unitarios por fila de TD §5.3 y tests de descriptor por fixture.

<a id="env-t011"></a>
#### ENV-T011 — Implementar valores multilínea entre comillas

| Campo | Valor |
|---|---|
| Épica | E2 Document Engine |
| Origen (requisito/decisión) | ENV-002; PRD US-02; TD §5.2 (invariante 2), §5.3 |
| Prioridad | P0 |
| Estimación | 6 h — incertidumbre alta (reglas de escape y cierre) |
| Depende de | ENV-T010 |
| Bloquea a | ENV-T012, ENV-T014, ENV-T021 |
| Sprint propuesto | S02 (2026-11-30 → 2026-12-04) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/syntax.rs |

**Descripción.** Detecta el cierre real de comillas según las reglas de escape de la especificación; un nodo de asignación abarca varias líneas físicas sin romper spans. Comillas sin cerrar se tratan según la regla aprobada (Raw acotado, sin tragarse el resto del archivo si así se decide).

**Criterios de aceptación**
- [ ] quoted-multiline.env y unclosed-quotes.env producen los nodos esperados.
- [ ] Terminadores mixtos dentro de un valor multilínea se conservan.
- [ ] Ninguna entrada provoca panic (incluido EOF dentro de comillas).

**Pruebas:** Tests unitarios de casos límite: escape antes de la comilla, comilla al final de archivo, CRLF interno.

<a id="env-t012"></a>
#### ENV-T012 — Implementar el serializador puro y el arnés de fidelidad byte a byte

| Campo | Valor |
|---|---|
| Épica | E2 Document Engine |
| Origen (requisito/decisión) | ENV-001; PRD §10, §14; TD §5.4, §14.1 |
| Prioridad | P0 |
| Estimación | 4 h |
| Depende de | ENV-T010, ENV-T011, ENV-T007 |
| Bloquea a | ENV-T013, ENV-T015, ENV-T016, ENV-T017, ENV-T018 |
| Sprint propuesto | S02 (2026-11-30 → 2026-12-04) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/document.rs (serialize), tests/fidelity.rs |

**Descripción.** `serialize(&EnvDocument) -> Vec<u8>` como función pura y tests/fidelity.rs que recorre todos los fixtures UTF-8 comparando bytes. Los mensajes de fallo informan solo offset y longitud de la primera diferencia, nunca contenido (PRD §10, logs sin secretos).

**Criterios de aceptación**
- [ ] serialize(parse(bytes)) == bytes para el 100 % de fixtures UTF-8 admitidos.
- [ ] Un fallo provocado muestra offset/longitud y no bytes del fixture.
- [ ] El serializador no normaliza BOM, EOL ni espacios.

**Pruebas:** tests/fidelity.rs; test negativo que verifica el formato del mensaje de fallo.

<a id="env-t013"></a>
#### ENV-T013 — Añadir pruebas de propiedades del roundtrip parser/serializador

| Campo | Valor |
|---|---|
| Épica | E2 Document Engine |
| Origen (requisito/decisión) | ENV-001; TD §3 (proptest opcional); decisión D-24 |
| Prioridad | P0 |
| Estimación | 4 h |
| Depende de | ENV-T012 |
| Bloquea a | Ninguna |
| Sprint propuesto | S02 (2026-11-30 → 2026-12-04) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | tests/properties.rs (o módulo de tests), fixtures de regresión |

**Descripción.** Generadores de líneas tipo dotenv y de bytes arbitrarios: roundtrip idéntico para UTF-8 válido, rechazo sin panic para UTF-8 inválido. Los casos mínimos encontrados se guardan como fixtures de regresión.

**Criterios de aceptación**
- [ ] Roundtrip verificado con un número de casos configurado (alto en local, reducido en CI).
- [ ] El parser nunca hace panic con entradas arbitrarias.
- [ ] Todo contraejemplo encontrado queda como fixture permanente.

**Pruebas:** Ejecución de proptest local y en CI.

**Riesgos / seguridad:** Añade dev-dependency (proptest); requiere aprobación D-24.

<a id="env-t014"></a>
#### ENV-T014 — Implementar ValueCodec por estilo de comillas

| Campo | Valor |
|---|---|
| Épica | E2 Document Engine |
| Origen (requisito/decisión) | ENV-002; PRD US-02 (conversión explícita); TD §5.3 (ValueCodec) |
| Prioridad | P0 |
| Estimación | 7 h — incertidumbre alta (dialectos y escapes) |
| Depende de | ENV-T006, ENV-T011 |
| Bloquea a | ENV-T016, ENV-T037, ENV-T049 |
| Sprint propuesto | S03 (2026-12-07 → 2026-12-11) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/syntax.rs o src/domain/codec.rs |

**Descripción.** `decode(raw, style) -> texto` y `encode(texto, style) -> Result<bytes, NeedsConversion o Unrepresentable>`. Si el valor original contiene secuencias ambiguas según la especificación, se marca como no editable en lugar de adivinar. Los valores no editados nunca se recodifican.

**Criterios de aceptación**
- [ ] Para cada valor de fixture editable, encode(decode(v)) reproduce los bytes originales.
- [ ] Valores no representables en el estilo actual devuelven NeedsConversion o Unrepresentable, nunca una codificación silenciosa.
- [ ] Valores con escapes ambiguos se marcan como no editables.

**Pruebas:** Tests tabulares por estilo (sin comillas, simple, doble, multilínea) con casos límite.

**Heredado del QA de ENV-T011 (2026-11-22):** la regla de SYNTAX §5.3 "CRLF cuenta como un solo carácter tras `\`" no es observable en el parser (solo decide dónde cierra la comilla, y el resultado es el mismo); se materializa en el decode. ValueCodec debe consumir `\` + CRLF como un único par y mostrarlo literal (`\` + salto), con un test que lo fije.

<a id="env-t015"></a>
#### ENV-T015 — Crear el ejemplo interno roundtrip de verificación de fidelidad

| Campo | Valor |
|---|---|
| Épica | E2 Document Engine |
| Origen (requisito/decisión) | TD §15 Hito 1 (salida: CLI de prueba interna) |
| Prioridad | P1 |
| Estimación | 2 h |
| Depende de | ENV-T012, ENV-T004 |
| Bloquea a | Ninguna |
| Sprint propuesto | S03 (2026-12-07 → 2026-12-11) |
| Estado | Backlog |
| Agentes | Implementador, Security Reviewer |
| Entregables / archivos | examples/roundtrip.rs |

**Descripción.** Binario de ejemplo (no distribuido) que lee un archivo real, informa número de nodos por tipo y resultado del roundtrip (OK u offset de diferencia). Nunca imprime valores.

**Criterios de aceptación**
- [ ] Sobre un archivo con secretos ficticios, la salida no contiene ningún valor.
- [ ] No se instala ni empaqueta con la aplicación.

**Pruebas:** Ejecución manual sobre fixtures y sobre un archivo con centinela.

### E3 — Editing Engine

<a id="env-t016"></a>
#### ENV-T016 — Implementar el comando SetValue con reemplazo mínimo

| Campo | Valor |
|---|---|
| Épica | E3 Editing Engine |
| Origen (requisito/decisión) | ENV-002; PRD US-02 (conservar fragmentos léxicos); TD §5.2 (invariante 6), §6.1 |
| Prioridad | P0 |
| Estimación | 6 h |
| Depende de | ENV-T014, ENV-T012 |
| Bloquea a | ENV-T019, ENV-T037 |
| Sprint propuesto | S03 (2026-12-07 → 2026-12-11) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/edit.rs |

**Descripción.** Sustituye solo los bytes del valor conservando prefijo, export, clave, separador, comillas, sufijo y terminador. Usa el ValueCodec; si devuelve NeedsConversion, la conversión de estilo solo ocurre con un parámetro explícito.

**Criterios de aceptación**
- [ ] Tras SetValue, todos los bytes fuera del rango del valor editado son idénticos.
- [ ] Sin parámetro de conversión, un valor no representable se rechaza.
- [ ] Funciona en valores multilínea y sobre claves duplicadas afectando solo a la ocurrencia elegida.

**Pruebas:** Tests unitarios por estilo de comillas, multilínea, Unicode y duplicados.

<a id="env-t017"></a>
#### ENV-T017 — Implementar el comando InsertAssignment al final del documento

| Campo | Valor |
|---|---|
| Épica | E3 Editing Engine |
| Origen (requisito/decisión) | ENV-008; PRD US-03 (nueva clave, salto de línea dominante, newline final) |
| Prioridad | P1 |
| Estimación | 5 h |
| Depende de | ENV-T012 |
| Bloquea a | ENV-T019, ENV-T045 |
| Sprint propuesto | S04 (2026-12-14 → 2026-12-18) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/edit.rs |

**Descripción.** Valida el nombre de clave y crea un nodo sintético al final usando el salto de línea dominante; si el archivo no terminaba en newline, la propiedad se mantiene según la regla de la especificación.

**Criterios de aceptación**
- [ ] Casos cubiertos: archivo vacío, LF, CRLF dominante, empate, sin newline final.
- [ ] Nombres inválidos se rechazan con error tipado sin valor.
- [ ] Ningún nodo existente cambia sus bytes salvo el terminador exigido por la regla documentada.

**Pruebas:** Tests unitarios por caso de terminador.

<a id="env-t018"></a>
#### ENV-T018 — Implementar el comando RemoveAssignment de una ocurrencia

| Campo | Valor |
|---|---|
| Épica | E3 Editing Engine |
| Origen (requisito/decisión) | ENV-008; PRD US-03 (eliminar solo la ocurrencia elegida) |
| Prioridad | P1 |
| Estimación | 3 h |
| Depende de | ENV-T012 |
| Bloquea a | ENV-T019, ENV-T045 |
| Sprint propuesto | S04 (2026-12-14 → 2026-12-18) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/edit.rs |

**Descripción.** Marca como eliminada una única ocurrencia por ID, con su terminador, conservando las demás apariciones de la misma clave y la regla de newline final.

**Criterios de aceptación**
- [ ] Eliminar una ocurrencia duplicada deja intactas las otras.
- [ ] Eliminar la última línea respeta la regla de newline final de la especificación.

**Pruebas:** Tests unitarios con duplicate-keys.env y no-trailing-newline.env.

<a id="env-t019"></a>
#### ENV-T019 — Implementar EditHistory con lotes transaccionales y cálculo de dirty

| Campo | Valor |
|---|---|
| Épica | E3 Editing Engine |
| Origen (requisito/decisión) | ENV-009, ENV-005, ENV-013; PRD US-03; TD §6.2 |
| Prioridad | P0 |
| Estimación | 6 h |
| Depende de | ENV-T016, ENV-T017, ENV-T018 |
| Bloquea a | ENV-T021, ENV-T029, ENV-T046, ENV-T068 |
| Sprint propuesto | S04 (2026-12-14 → 2026-12-18) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/history.rs |

**Descripción.** apply/undo/redo con pilas en memoria; Batch transaccional (se valida todo antes de mutar); IDs de nodo estables entre undo/redo; dirty calculado comparando la serialización actual con el último snapshot confirmado, no por la pila.

**Criterios de aceptación**
- [ ] Undo de un lote restaura exactamente los bytes previos; redo restaura exactamente el lote.
- [ ] Undo después de guardar marca el documento como modificado.
- [ ] Un lote con una operación inválida no altera el documento.

**Pruebas:** Tests unitarios de secuencias apply/undo/redo y de lote fallido.

<a id="env-t020"></a>
#### ENV-T020 — Implementar el diagnóstico de claves duplicadas

| Campo | Valor |
|---|---|
| Épica | E3 Editing Engine |
| Origen (requisito/decisión) | ENV-019, ENV-010; PRD US-07 |
| Prioridad | P1 |
| Estimación | 2 h |
| Depende de | ENV-T010 |
| Bloquea a | ENV-T021, ENV-T066 |
| Sprint propuesto | S03 (2026-12-07 → 2026-12-11) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/document.rs o src/domain/compare.rs |

**Descripción.** Función pura que agrupa ocurrencias por clave con su recuento; solo diagnóstico, nunca representación primaria.

**Criterios de aceptación**
- [ ] duplicate-keys.env devuelve las claves repetidas con número de ocurrencias e IDs.
- [ ] No determina qué ocurrencia prevalece.

**Pruebas:** Tests unitarios.

<a id="env-t021"></a>
#### ENV-T021 — Implementar el núcleo de pegado de asignaciones (PastePlan)

| Campo | Valor |
|---|---|
| Épica | E3 Editing Engine |
| Origen (requisito/decisión) | ENV-010; PRD US-03; TD §6.3 |
| Prioridad | P1 |
| Estimación | 7 h |
| Depende de | ENV-T011, ENV-T019, ENV-T020 |
| Bloquea a | ENV-T050, ENV-T068, ENV-T070 |
| Sprint propuesto | S06 (2026-12-28 → 2027-01-01) |
| Estado | Backlog |
| Agentes | Implementador, QA, Security Reviewer |
| Entregables / archivos | src/domain/edit.rs o src/services/document_service.rs (paste) |

**Descripción.** Parsea el texto pegado con el mismo parser en un documento temporal; clasifica entradas válidas e inválidas (con número de línea); detecta claves existentes (0, 1 o n ocurrencias); produce un PastePlan que exige decisiones (añadir, reemplazar ocurrencia elegida, cancelar) y genera un único Batch solo cuando todas están resueltas. Inserción parcial solo con confirmación explícita.

**Criterios de aceptación**
- [ ] Nunca se reemplaza automáticamente por nombre; con varias ocurrencias no se infiere cuál.
- [ ] Con líneas no interpretables, el plan no es aplicable sin la confirmación de insertar solo las válidas.
- [ ] Asignaciones multilínea pegadas se conservan; el lote resultante se deshace en una operación.
- [ ] El contenido pegado se trata como datos; nada se expande.

**Pruebas:** Tests unitarios por cada viñeta de PRD US-03 sobre pegado.

<a id="env-t022"></a>
#### ENV-T022 — Implementar la heurística de secretos por nombre de clave

| Campo | Valor |
|---|---|
| Épica | E3 Editing Engine |
| Origen (requisito/decisión) | ENV-012; PRD §8.3 |
| Prioridad | P1 |
| Estimación | 2 h |
| Depende de | ENV-T008 |
| Bloquea a | ENV-T048 |
| Sprint propuesto | S03 (2026-12-07 → 2026-12-11) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/secrets.rs (o similar) |

**Descripción.** Clasificador por tokens del nombre de clave (SECRET, TOKEN, PASSWORD, KEY y similares listados en una constante), sin mirar valores. Documentado como heurístico.

**Criterios de aceptación**
- [ ] Tabla de casos positivos y negativos documentada y probada.
- [ ] La función no recibe ni inspecciona valores.

**Pruebas:** Tests tabulares.

<a id="env-t023"></a>
#### ENV-T023 — Implementar la comparación de conjuntos de claves entre documentos

| Campo | Valor |
|---|---|
| Épica | E3 Editing Engine |
| Origen (requisito/decisión) | ENV-018, ENV-005 (revisar conflicto); PRD US-07; TD §7.3 |
| Prioridad | P0 |
| Estimación | 3 h |
| Depende de | ENV-T010 |
| Bloquea a | ENV-T032, ENV-T065 |
| Sprint propuesto | S03 (2026-12-07 → 2026-12-11) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/compare.rs |

**Descripción.** Función pura que devuelve Faltan / Solo en actual / En ambos (con recuento de ocurrencias) entre dos documentos, sin comparar ni exponer valores. Reutilizada por la revisión de conflictos (T032) y la comparación con .env.example (T065).

**Criterios de aceptación**
- [ ] Resultados correctos con duplicados y documentos vacíos.
- [ ] La salida no contiene valores.

**Pruebas:** Tests unitarios.

<a id="env-t024"></a>
#### ENV-T024 — Implementar la inferencia de etiqueta de entorno por nombre de archivo

| Campo | Valor |
|---|---|
| Épica | E3 Editing Engine |
| Origen (requisito/decisión) | ENV-017; PRD US-07; TD §10.4 |
| Prioridad | P2 |
| Estimación | 2 h |
| Depende de | ENV-T002 |
| Bloquea a | ENV-T064 |
| Sprint propuesto | S01 (2026-11-23 → 2026-11-27) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/domain/project.rs (o environment.rs) |

**Descripción.** Tokens production/prod -> Production, staging/stage -> Staging, development/dev/local -> Development; sin coincidencia o con coincidencias múltiples -> Other.

**Criterios de aceptación**
- [ ] .env.production y prod.env -> Production; .env.local -> Development; .env.example y .env -> Other.
- [ ] Nombres ambiguos (p. ej. .env.prod.local) -> Other.

**Pruebas:** Tests tabulares.

### E4 — Filesystem & Safety

<a id="env-t025"></a>
#### ENV-T025 — Spike: semántica real de guardado condicionado con Gio en sistema local

| Campo | Valor |
|---|---|
| Épica | E4 Filesystem & Safety |
| Origen (requisito/decisión) | ENV-004; PRD US-05, §8.7-8.8, §13; TD §8.2, §16; decisión D-12 |
| Prioridad | P0 |
| Estimación | 6 h — incertidumbre alta |
| Depende de | ENV-T001, ENV-T002 |
| Bloquea a | ENV-T027, ENV-T028, ENV-T054 |
| Sprint propuesto | S02 (2026-11-30 → 2026-12-04) |
| Estado | Backlog |
| Agentes | Explorador, Implementador, Security Reviewer |
| Entregables / archivos | Matriz de resultados y algoritmo de guardado recomendado anotados en DECISIONS.md (D-12) y en la descripción de T028 |

**Descripción.** Spike con timebox de 6 h sobre g_file_replace_contents con etag: base real del etag local, comportamiento si el archivo cambia entre lectura y reemplazo, conservación de modo 0600/0644 y propietario, archivo sin escritura, directorio sin permiso de creación (posible caída a escritura no atómica), enlaces simbólicos, enlaces duros y make_backup=false. Código desechable.

**Criterios de aceptación**
- [ ] Matriz completa de los casos listados con resultado observado y versión de GLib.
- [ ] Declaración explícita de la ventana de carrera residual (TOCTOU) y de cuándo hay que bloquear el guardado.
- [ ] Criterio de salida: estrategia propuesta para T028 o escalado al usuario si no existe garantía aceptable.

**Pruebas:** Programa de prueba desechable con tempfile; evidencia de cada caso.

**Riesgos / seguridad:** Ruta crítica de P0. Si la garantía no se cumple, el producto debe bloquear el guardado y ofrecer Guardar como.

<a id="env-t026"></a>
#### ENV-T026 — Definir el puerto DocumentStorage y un almacenamiento falso para pruebas

| Campo | Valor |
|---|---|
| Épica | E4 Filesystem & Safety |
| Origen (requisito/decisión) | ENV-004; TD ADR-010, §8.1; decisión D-25 (hash) |
| Prioridad | P0 |
| Estimación | 4 h |
| Depende de | ENV-T008, ENV-T004 |
| Bloquea a | ENV-T027, ENV-T029 |
| Sprint propuesto | S04 (2026-12-14 → 2026-12-18) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/services/ (puerto), tests de apoyo |

**Descripción.** Trait DocumentStorage, ResourceRef (URI), ReadSnapshot, DiskVersion (hash + etag + tamaño + mtime), StorageCapabilities y StorageError sin valores. FakeStorage configurable para simular cambios externos, fallos, ausencia, solo lectura y falta de reemplazo condicional.

**Criterios de aceptación**
- [ ] El núcleo y los servicios compilan sin GTK usando el puerto.
- [ ] FakeStorage permite reproducir cada fila de TD §14.2 en tests puros.
- [ ] StorageError no contiene contenido de archivo.

**Pruebas:** Tests unitarios del FakeStorage.

<a id="env-t027"></a>
#### ENV-T027 — Implementar la lectura segura de archivos en GioStorage

| Campo | Valor |
|---|---|
| Épica | E4 Filesystem & Safety |
| Origen (requisito/decisión) | ENV-004, ENV-006; PRD US-01 (errores legibles), §8.5, §8.7; TD §8.2 paso 3 |
| Prioridad | P0 |
| Estimación | 6 h |
| Depende de | ENV-T026, ENV-T025 |
| Bloquea a | ENV-T028, ENV-T031, ENV-T034, ENV-T043, ENV-T057 |
| Sprint propuesto | S04 (2026-12-14 → 2026-12-18) |
| Estado | Backlog |
| Agentes | Implementador, QA, Security Reviewer |
| Entregables / archivos | src/infrastructure/gio_storage.rs, src/infrastructure/filesystem.rs |

**Descripción.** Apertura por GFile/URI con consulta NOFOLLOW_SYMLINKS: rechazo de enlaces simbólicos y no regulares, detección de solo lectura, etag, lectura asíncrona fuera del hilo principal, hash e identidad canónica. Nunca crea archivos.

**Criterios de aceptación**
- [ ] Archivo inexistente, directorio, FIFO, sin permiso de lectura y enlace simbólico devuelven errores distintos y legibles.
- [ ] Un archivo sin permiso de escritura se abre como solo lectura.
- [ ] Ninguna ruta de error crea o modifica archivos.

**Pruebas:** Tests de integración con tempfile para cada caso (sin display).

**Heredado de la revisión de seguridad de ENV-T010 (2026-11-22):** el parser usa ~165× el tamaño del archivo en memoria (medido: 20 MiB de `\n` → 3,4 GB RSS; la revisión de ENV-T011 midió un pico de heap de ~190–310× según el tamaño). Aplicar un límite de tamaño antes de leer (`metadata` + lectura con `take(limit + 1)`, propuesta 8–16 MiB; por encima, rechazar o abrir en modo protegido con mensaje claro) y parsear fuera del hilo principal. Opcional en el núcleo: `split_lines` como iterador, `Vec::with_capacity` y clave como rango.

**Heredado de la revisión de seguridad del spike ENV-T025 (2026-11-22):** `base` debe capturar hash, `etag::value`, **dev + inodo**, uid, gid, modo, nlink, si es symlink, propietario/modo/sticky del directorio padre, presencia de ACL (`system.posix_acl_access`, y `system.posix_acl_default` en el padre) y etiqueta SELinux. Leer sin seguir symlinks y comprobar que la identidad del descriptor leído (`fstat`) coincide con la consultada.

<a id="env-t028"></a>
#### ENV-T028 — Implementar replace_if_unchanged con verificación posterior

| Campo | Valor |
|---|---|
| Épica | E4 Filesystem & Safety |
| Origen (requisito/decisión) | ENV-004; PRD US-05, §8.4, §8.7-8.8; TD §8.2 |
| Prioridad | P0 |
| Estimación | 7 h — incertidumbre alta |
| Depende de | ENV-T027, ENV-T025 |
| Bloquea a | ENV-T030, ENV-T067 |
| Sprint propuesto | S05 (2026-12-21 → 2026-12-25) |
| Estado | Backlog |
| Agentes | Implementador, QA, Security Reviewer |
| Entregables / archivos | src/infrastructure/gio_storage.rs |

**Descripción.** Según la estrategia del spike T025: relee y compara hash y etag con la versión esperada inmediatamente antes, reemplaza con etag y make_backup=false, relee tras escribir para confirmar contenido y modo, y declara capacidades honestas. Cualquier duda => Conflict o error, nunca éxito.

**Criterios de aceptación**
- [ ] Un cambio externo entre lectura y guardado produce Conflict sin escribir.
- [ ] Éxito solo se informa tras releer y verificar el hash del contenido escrito.
- [ ] Si el modo de permisos no se conserva, se informa según la política aprobada (D-12).
- [ ] No se crea ningún archivo de respaldo.

**Pruebas:** Tests de integración con tempfile; la matriz completa se cubre en T067.

**Riesgos / seguridad:** Ventana TOCTOU residual (R-01); documentar la limitación, no prometer atomicidad concurrente.

**Estrategia del spike ENV-T025 (2026-11-22, [informe](spikes/ENV-T025-gio-save.md) §4):**
1. `lstat`: rechazar symlinks por defecto (Gio escribe el target en el sitio; un colgante crea el target).
2. Pre-vuelo de ruta atómica: archivo regular con `nlink == 1`, directorio padre escribible, archivo escribible y uid/gid conservables. Si falla, bloquear Guardar y ofrecer Guardar como (D-12.3): Gio cae a `ftruncate` + escritura en el sitio sin avisar y no se puede detectar a posteriori.
3. Hash + etag justo antes (el etag local es solo el mtime en ns: ciego a mtime restaurado y a chmod; el hash es obligatorio).
4. `File::replace(Some(etag), make_backup = false, FileCreateFlags::PRIVATE)` + `write_all` + `close`, **no** `replace_contents` (ver revisión de seguridad); nunca `REPLACE_DESTINATION` (pierde modo y grupo, rompe enlaces).
5. Verificación posterior contra el `lstat` fresco del pre-vuelo: contenido, inodo distinto del base, modo/uid/gid; error visible si difieren (D-12.2), sin `chmod` automático hacia un modo más permisivo.
- El etag solo se comprueba al abrir el stream: un cambio externo entre el open y el rename se pierde sin error. Se documenta como "detección de cambios best-effort, no exclusión mutua" (D-12.1).
- La ruta rename pierde xattr de usuario, ACL y la etiqueta SELinux personalizada (pendiente de decisión).
- T027 debe capturar en `base`: hash, `etag::value`, `unix::mode/uid/gid/inode/nlink` y si es symlink.

**Revisión de seguridad de la estrategia (2026-11-22, cybersecurity-engineer) — corrige la recomendación del spike:**
- **B1 (alta, confirmado):** `replace_contents` cierra el stream tras un error de escritura y GLib hace el `rename`: el original queda sustituido por el contenido truncado (probado con `RLIMIT_FSIZE`). Soltar el stream sin cerrarlo hace lo mismo. Usar `File::replace` con un `Cancellable` propio; ante cualquier error, `cancel()` y `close(Some(&cancellable))` (GLib borra el temporal y no renombra). Un guard con `Drop` garantiza que ningún camino suelta el stream sin ese cierre.
- **B2 (media, confirmado):** con `NONE`, el temporal `.goutputstream-XXXXXX` se crea con `0666 & ~umask` antes del `fchmod`; otro usuario con acceso al directorio puede abrirlo y leer los secretos. Usar `FileCreateFlags::PRIVATE` **siempre** (conserva 0600/0644 en archivos existentes, casos 3e/3f del spike).
- **B3 (media):** el pre-vuelo exige `uid == euid` **y** gid en los grupos del proceso (no "o"); si no, Gio cae a escritura en el sitio.
- **B4 (media, confirmado):** si el archivo se borra entre la comprobación y el guardado, Gio lo recrea ignorando el etag. "No existe" en el paso 3 ⇒ conflicto; inodo nuevo inesperado ⇒ no declarar éxito.
- **B5 (media, riesgo residual):** Gio abre con `O_NOFOLLOW` pero ante `ELOOP` reabre siguiendo el enlace. En un directorio escribible por terceros, un symlink o archivo ajeno colocado tras el pre-vuelo recibe los secretos. Mitigación propuesta: pre-vuelo de "directorio seguro" (padre del euid o root y no escribible por grupo/otros salvo sticky) o un backend local basado en descriptores (`openat`/`renameat` con `rustix`). **Decidido (2026-11-22): pre-vuelo de directorio seguro en v0.1**; si falla, bloquear Guardar y ofrecer Guardar como. `rustix` queda para después, solo si hace falta.
- **B6–B9 (media/baja):** no restaurar nunca a un modo más permisivo; comparar ACL y `security.selinux` en la verificación posterior (la ruta rename hereda la ACL por defecto del directorio); temporales huérfanos tras crash y sin `fsync` del directorio; Guardar como pasa por el mismo `lstat` y pre-vuelo.
- **D-12 y D-34 aprobadas el 2026-11-22** con estas correcciones (ver DECISIONS.md); xattr/ACL/SELinux se comparan y un cambio es error visible (no se copian en v0.1).
- Criterios añadidos: sin `replace_contents`; `PRIVATE` en toda llamada; pre-vuelo uid **y** gid, directorio seguro, repetido justo antes de `replace`; archivo inexistente ⇒ conflicto; verificación contra el `lstat` fresco (dev+ino, uid, gid, modo, ACL, SELinux).

<a id="env-t029"></a>
#### ENV-T029 — Implementar DocumentSession y la máquina de estados del documento

| Campo | Valor |
|---|---|
| Épica | E4 Filesystem & Safety |
| Origen (requisito/decisión) | ENV-005, ENV-013; PRD US-05; TD ADR-005, §7.1, §7.2 |
| Prioridad | P0 |
| Estimación | 7 h |
| Depende de | ENV-T019, ENV-T026 |
| Bloquea a | ENV-T030, ENV-T031, ENV-T032, ENV-T035, ENV-T036, ENV-T041 |
| Sprint propuesto | S04 (2026-12-14 → 2026-12-18) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/services/document_service.rs (sesión), src/domain/ |

**Descripción.** Sesión por archivo con banderas separadas dirty/conflict/read-only/missing/saving y transiciones de TD §7.2 ante eventos (edición, cambio externo igual o distinto, borrado, guardado, recargar y descartar confirmado, guardar como). Lógica pura probada con FakeStorage.

**Criterios de aceptación**
- [ ] Cada fila de TD §7.2 tiene al menos un test.
- [ ] Ninguna transición descarta ediciones locales sin el evento explícito de recargar y descartar.
- [ ] dirty y conflict pueden coexistir y se exponen por separado.

**Pruebas:** Tests unitarios por transición.

<a id="env-t030"></a>
#### ENV-T030 — Implementar la orquestación de Guardar y Guardar como

| Campo | Valor |
|---|---|
| Épica | E4 Filesystem & Safety |
| Origen (requisito/decisión) | ENV-004, ENV-005; PRD US-05; TD §7.2, §8.2; decisión D-27 |
| Prioridad | P0 |
| Estimación | 5 h |
| Depende de | ENV-T029, ENV-T028 |
| Bloquea a | ENV-T040, ENV-T069, ENV-T070 |
| Sprint propuesto | S05 (2026-12-21 → 2026-12-25) |
| Estado | Backlog |
| Agentes | Implementador, QA, Security Reviewer |
| Entregables / archivos | src/services/document_service.rs |

**Descripción.** save: serializar, comprobar capacidades, replace_if_unchanged, actualizar base y limpiar dirty; si falla, conservar documento e historial. save_as: escribir en un recurso nuevo sin tocar el destino conflictivo; semántica posterior de la sesión según D-27.

**Criterios de aceptación**
- [ ] Un fallo de guardado deja documento, historial y estado dirty intactos.
- [ ] Guardar como nunca modifica el archivo original en conflicto.
- [ ] Un guardado con capacidades insuficientes se bloquea y sugiere Guardar como.

**Pruebas:** Tests con FakeStorage para éxito, conflicto, fallo de E/S y capacidades insuficientes.

<a id="env-t031"></a>
#### ENV-T031 — Implementar el adaptador FileMonitor con coalescencia y verificación por hash

| Campo | Valor |
|---|---|
| Épica | E4 Filesystem & Safety |
| Origen (requisito/decisión) | ENV-005; PRD US-05; TD §7.3 |
| Prioridad | P0 |
| Estimación | 6 h — incertidumbre media-alta (variabilidad de eventos) |
| Depende de | ENV-T029, ENV-T027 |
| Bloquea a | ENV-T042, ENV-T069 |
| Sprint propuesto | S05 (2026-12-21 → 2026-12-25) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/infrastructure/file_monitor.rs |

**Descripción.** Monitor Gio por sesión (con seguimiento de movimientos), debounce corto definido como constante, relectura y comparación de hash con la base, supresión del eco del propio guardado solo por igualdad de snapshot, paso a Missing ante borrado/movimiento y a ReadOnly ante pérdida de permiso; rearmado del monitor cuando otro editor guarda por renombrado.

**Criterios de aceptación**
- [ ] Eventos repetidos producen una única comprobación/recarga.
- [ ] El guardado propio no se marca como cambio externo.
- [ ] Un guardado externo por renombrado (estilo vim/sed -i) sigue detectándose.

**Pruebas:** Tests de integración con bucle principal GLib (sin display); matriz completa en T069.

<a id="env-t032"></a>
#### ENV-T032 — Implementar el resumen de revisión de conflicto sin valores

| Campo | Valor |
|---|---|
| Épica | E4 Filesystem & Safety |
| Origen (requisito/decisión) | ENV-005; PRD US-05 (Revisar conflicto); TD §7.3 |
| Prioridad | P0 |
| Estimación | 4 h |
| Depende de | ENV-T029, ENV-T023 |
| Bloquea a | ENV-T042 |
| Sprint propuesto | S06 (2026-12-28 → 2027-01-01) |
| Estado | Backlog |
| Agentes | Implementador, QA, Security Reviewer |
| Entregables / archivos | src/services/document_service.rs o src/domain/compare.rs |

**Descripción.** Compara la versión local en memoria con el snapshot de disco: claves añadidas, eliminadas y cambiadas (comparando valores solo en memoria) y diferencia de líneas. La salida solo contiene nombres de clave y estado, nunca valores.

**Criterios de aceptación**
- [ ] El resumen distingue añadidas/eliminadas/cambiadas y cuenta líneas distintas.
- [ ] Ningún valor aparece en la estructura de salida ni en su Debug.

**Pruebas:** Tests unitarios con centinela.

### E5 — Quick Edit

<a id="env-t033"></a>
#### ENV-T033 — Inicializar la aplicación GTK4/libadwaita y la ventana principal

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-006; TD §9, §10.1; decisiones D-05, D-07, D-09 |
| Prioridad | P0 |
| Estimación | 6 h |
| Depende de | ENV-T001, ENV-T002, ENV-T004 |
| Bloquea a | ENV-T005, ENV-T034, ENV-T036, ENV-T056, ENV-T077, ENV-T078 |
| Sprint propuesto | S07 (2027-01-04 → 2027-01-08) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/main.rs, src/application.rs, src/ui/window.rs, resources/resources.gresource.xml, resources/ui/window.ui |

**Descripción.** Dependencias gtk4/libadwaita según T001; AdwApplication con app ID provisional y HANDLES_OPEN; compilación de GResource; AdwApplicationWindow con AdwToastOverlay, AdwToolbarView, AdwHeaderBar, AdwBanner y stack raíz; inicialización de tracing con la política sin contenido; helper de i18n (identidad hasta T080).

**Criterios de aceptación**
- [ ] La app arranca en nativo sin GTK-CRITICAL en consola.
- [ ] Un segundo lanzamiento activa la instancia existente.
- [ ] Todas las cadenas visibles pasan por el helper de i18n.

**Pruebas:** Prueba manual de arranque; test GTK de humo si hay display.

**Heredado de la revisión de seguridad de ENV-T004 (2026-11-21):** el panic hook descarta el payload del panic y registra solo la ubicación (los panics de std pueden citar contenido); prohibir `assert_eq!`/`debug_assert_eq!` en código de producción (`disallowed-macros`).

<a id="env-t034"></a>
#### ENV-T034 — Implementar la CLI y el manejo de apertura de archivos

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-006; PRD US-01, §7.1; TD §9 |
| Prioridad | P1 |
| Estimación | 5 h |
| Depende de | ENV-T033, ENV-T027 |
| Bloquea a | ENV-T035, ENV-T043, ENV-T052, ENV-T081 |
| Sprint propuesto | S07 (2027-01-04 → 2027-01-08) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/cli.rs, src/application.rs |

**Descripción.** `envy`, `envy <ruta relativa>`, `envy <ruta absoluta>` y `--help` mediante GApplication; resolución de rutas relativas contra el directorio del invocador incluso si la instancia primaria ya existe; errores legibles para inexistente, inaccesible, no regular o directorio; jamás crea archivos.

**Criterios de aceptación**
- [ ] envy .env abre el archivo del directorio actual del terminal invocador.
- [ ] envy sin argumentos muestra bienvenida (Workspace se añade en T061).
- [ ] Rutas inválidas muestran un diálogo legible y no crean archivos.

**Pruebas:** Pruebas manuales scriptadas en terminal; tests unitarios de resolución de rutas.

<a id="env-t035"></a>
#### ENV-T035 — Implementar el registro de sesiones y ventanas por documento

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-006; PRD US-01 (no descartar cambios al abrir otro archivo); TD §9; decisión D-15 |
| Prioridad | P1 |
| Estimación | 4 h |
| Depende de | ENV-T034, ENV-T029 |
| Bloquea a | ENV-T044, ENV-T060 |
| Sprint propuesto | S07 (2027-01-04 → 2027-01-08) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/application.rs, src/services/document_service.rs |

**Descripción.** Registro de sesiones por URI canónica: abrir otro archivo crea una ventana/sesión independiente (según D-15); abrir un archivo ya abierto enfoca su ventana; ninguna apertura afecta a sesiones con cambios.

**Criterios de aceptación**
- [ ] Abrir un segundo archivo con el primero modificado no altera el primero.
- [ ] Abrir dos veces el mismo archivo enfoca la ventana existente.

**Pruebas:** Pruebas manuales; tests unitarios del registro.

<a id="env-t036"></a>
#### ENV-T036 — Implementar EditorView con filas por tipo de nodo

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-002, ENV-003; PRD §7.3; TD §10.2 |
| Prioridad | P0 |
| Estimación | 7 h — incertidumbre media (rendimiento con archivos grandes) |
| Depende de | ENV-T033, ENV-T029 |
| Bloquea a | ENV-T037, ENV-T039, ENV-T040, ENV-T041, ENV-T045, ENV-T047, ENV-T048, ENV-T049, ENV-T060, ENV-T066 |
| Sprint propuesto | S07 (2027-01-04 → 2027-01-08) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/editor.rs, src/ui/assignment_row.rs, resources/ui/editor.ui, resources/ui/assignment_row.ui |

**Descripción.** Lista de filas construida desde la sesión: fila de asignación (clave + control de valor), filas de comentario, vacía y Raw de solo lectura, seleccionables, con etiqueta Sin modificar / Sintaxis no editable; IDs de fila = IDs de nodo; refresco tras cambios; contador de variables.

**Criterios de aceptación**
- [ ] Todos los fixtures se muestran con el tipo de fila correcto, incluidos duplicados.
- [ ] Las filas Raw y de comentario no ofrecen edición estructurada.
- [ ] El refresco conserva la posición de desplazamiento y el foco razonablemente.

**Pruebas:** Prueba manual con todos los fixtures; test GTK de construcción de filas si hay display.

<a id="env-t037"></a>
#### ENV-T037 — Implementar la edición de valores de una línea con conversión explícita

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-002; PRD US-02 (conversión explícita o rechazo) |
| Prioridad | P0 |
| Estimación | 6 h |
| Depende de | ENV-T036, ENV-T039, ENV-T016, ENV-T014 |
| Bloquea a | ENV-T038, ENV-T046 |
| Sprint propuesto | S08 (2027-01-11 → 2027-01-15) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/assignment_row.rs, src/ui/editor.rs |

**Descripción.** La edición de la fila se confirma como un SetValue según la política de T039. NeedsConversion abre un AdwAlertDialog que ofrece convertir el estilo o cancelar; Unrepresentable se rechaza con mensaje; valores no editables se muestran en solo lectura.

**Criterios de aceptación**
- [ ] Editar DEBUG=true a false cambia solo esos bytes al guardar.
- [ ] Un valor no representable nunca se guarda sin confirmación de conversión.
- [ ] El estado pasa a Modificado tras confirmar la edición.

**Pruebas:** Prueba manual con fixtures de cada estilo; tests de servicio ya cubren bytes.

<a id="env-t038"></a>
#### ENV-T038 — Implementar la fila expandible para valores multilínea

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-002; PRD §7.3; TD §10.2 |
| Prioridad | P0 |
| Estimación | 5 h |
| Depende de | ENV-T037 |
| Bloquea a | ENV-T048, ENV-T053 |
| Sprint propuesto | S08 (2027-01-11 → 2027-01-15) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/assignment_row.rs |

**Descripción.** GtkTextView dentro de una fila expandible con ajuste de línea y altura máxima con desplazamiento; misma política de confirmación; resumen de número de líneas cuando está plegada.

**Criterios de aceptación**
- [ ] Editar un valor multilínea conserva comillas y terminadores del resto del archivo.
- [ ] La fila no expande toda la ventana con valores largos.

**Pruebas:** Prueba manual con quoted-multiline.env.

<a id="env-t039"></a>
#### ENV-T039 — Spike: política única de undo y granularidad de confirmación de edición

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-009; TD §6.2 (dos historiales contradictorios); decisión D-13 |
| Prioridad | P1 |
| Estimación | 4 h |
| Depende de | ENV-T036 |
| Bloquea a | ENV-T037, ENV-T046 |
| Sprint propuesto | S07 (2027-01-04 → 2027-01-08) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | Propuesta D-13 en DECISIONS.md con notas del prototipo |

**Descripción.** Spike con timebox de 4 h. Hipótesis: desactivar el undo local de los widgets de texto y enrutar Ctrl+Z/Ctrl+Shift+Z al historial de Envy a nivel de ventana, confirmando cada edición de campo como un SetValue, ofrece un comportamiento coherente con foco en campos.

**Criterios de aceptación**
- [ ] Escenarios de foco probados manualmente: campo con foco, fila multilínea, diálogo abierto.
- [ ] Criterio de salida: política elegida y documentada; sin dos historiales activos a la vez.

**Pruebas:** Pruebas manuales de teclado documentadas.

<a id="env-t040"></a>
#### ENV-T040 — Implementar las acciones Guardar y Guardar como en la UI

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-004; PRD §7.3, §7.4; US-05 |
| Prioridad | P0 |
| Estimación | 5 h |
| Depende de | ENV-T036, ENV-T030 |
| Bloquea a | ENV-T042, ENV-T044 |
| Sprint propuesto | S08 (2027-01-11 → 2027-01-15) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/window.rs, src/ui/editor.rs |

**Descripción.** Acción win.save con Ctrl+S, botón deshabilitado sin cambios o en solo lectura, Ctrl+S con comportamiento consistente, Guardar como con GtkFileDialog, toast Guardado, errores comprensibles sin valores, bloqueo de guardados concurrentes durante Saving.

**Criterios de aceptación**
- [ ] El toast Guardado solo aparece tras confirmación del servicio.
- [ ] Un error de guardado mantiene el estado Modificado y el contenido.
- [ ] Ctrl+S sin cambios no escribe en disco.

**Pruebas:** Prueba manual con archivo 0600 y archivo sin permiso de escritura.

<a id="env-t041"></a>
#### ENV-T041 — Implementar indicadores de estado y modos de solo lectura

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-013; PRD §7.2-7.3; TD §10.3 |
| Prioridad | P1 |
| Estimación | 4 h |
| Depende de | ENV-T036, ENV-T029 |
| Bloquea a | ENV-T053 |
| Sprint propuesto | S07 (2027-01-04 → 2027-01-08) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/window.rs, src/ui/editor.rs |

**Descripción.** Estados Sin cambios, Modificado, Conflicto, Solo lectura y Archivo no disponible con texto + icono (no solo color); el modo solo lectura deshabilita la edición; UTF-8 no válido muestra una página de estado que explica que la edición estructurada no está disponible.

**Criterios de aceptación**
- [ ] Cada estado es identificable por texto y por lector de pantalla.
- [ ] invalid-utf8.bin abre una página explicativa sin reescribir el archivo.

**Pruebas:** Prueba manual por estado; test GTK si hay display.

<a id="env-t042"></a>
#### ENV-T042 — Implementar el banner de conflicto y sus flujos de resolución

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-005; PRD US-05, §7.3; TD §7.2, §10.3; decisión D-14 |
| Prioridad | P0 |
| Estimación | 6 h |
| Depende de | ENV-T040, ENV-T031, ENV-T032 |
| Bloquea a | ENV-T053, ENV-T082 |
| Sprint propuesto | S08 (2027-01-11 → 2027-01-15) |
| Estado | Backlog |
| Agentes | Implementador, QA, Security Reviewer |
| Entregables / archivos | src/ui/conflict_view.rs, src/ui/window.rs |

**Descripción.** AdwBanner persistente con Recargar y descartar mis cambios (con confirmación), Guardar como… y Revisar conflicto (diálogo con el resumen de T032, sin valores). Banner de archivo no disponible con Guardar como. Recarga automática con toast Recargado cuando no hay cambios. Sobrescritura no ofrecida salvo aprobación de D-14.

**Criterios de aceptación**
- [ ] Con cambios locales y modificación externa, el banner persiste hasta resolver y no se pierde nada.
- [ ] Recargar y descartar exige confirmación explícita.
- [ ] Revisar conflicto no muestra valores por defecto.

**Pruebas:** Prueba manual con edición externa concurrente (editor de texto y script).

<a id="env-t043"></a>
#### ENV-T043 — Implementar el aviso de enlace simbólico y la selección explícita del destino

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | PRD §8.5; TD §8.2 (política de symlinks) |
| Prioridad | P0 |
| Estimación | 3 h |
| Depende de | ENV-T034, ENV-T027 |
| Bloquea a | ENV-T053 |
| Sprint propuesto | S09 (2027-01-18 → 2027-01-22) |
| Estado | Backlog |
| Agentes | Implementador, QA, Security Reviewer |
| Entregables / archivos | src/ui/window.rs, src/application.rs |

**Descripción.** Al abrir un enlace simbólico se informa de que no se edita por defecto y se ofrece abrir el selector para elegir expresamente el archivo destino regular, sujeto a permisos.

**Criterios de aceptación**
- [ ] Abrir un enlace simbólico nunca lo reemplaza por un archivo regular.
- [ ] El usuario puede seleccionar el destino mediante el selector y editarlo.

**Pruebas:** Prueba manual con enlace simbólico a un .env; regresión en T067.

<a id="env-t044"></a>
#### ENV-T044 — Implementar la protección al cerrar con cambios sin guardar

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | PRD §10 (cero pérdidas sin acción expresa), US-01; decisión D-16 |
| Prioridad | P0 |
| Estimación | 4 h |
| Depende de | ENV-T040, ENV-T035 |
| Bloquea a | ENV-T053 |
| Sprint propuesto | S09 (2027-01-18 → 2027-01-22) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/window.rs, src/application.rs |

**Descripción.** close-request de ventana y salida de la aplicación con diálogo Guardar / Descartar / Cancelar; si el guardado falla o hay conflicto, el cierre se aborta.

**Criterios de aceptación**
- [ ] Cerrar una ventana modificada nunca descarta cambios sin elección explícita.
- [ ] Un guardado fallido durante el cierre mantiene la ventana abierta.

**Pruebas:** Prueba manual de cierre con Ctrl+W, botón de cierre y salida de aplicación.

<a id="env-t045"></a>
#### ENV-T045 — Implementar agregar y eliminar asignaciones en la UI

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-008; PRD US-03, §7.4 (Ctrl+N) |
| Prioridad | P1 |
| Estimación | 5 h |
| Depende de | ENV-T036, ENV-T017, ENV-T018 |
| Bloquea a | ENV-T046, ENV-T065, ENV-T072 |
| Sprint propuesto | S08 (2027-01-11 → 2027-01-15) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/editor.rs, src/ui/assignment_row.rs |

**Descripción.** Ctrl+N abre un diálogo con validación del nombre de clave y error anunciado accesiblemente; la acción eliminar de cada fila elimina solo esa ocurrencia; ambas acciones son deshacibles.

**Criterios de aceptación**
- [ ] Un nombre inválido muestra un error asociado al campo y anunciado.
- [ ] Eliminar una clave duplicada elimina solo la fila elegida.

**Pruebas:** Prueba manual; test GTK del diálogo si hay display.

<a id="env-t046"></a>
#### ENV-T046 — Integrar undo/redo del historial en la UI

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-009; PRD US-03, §7.4 |
| Prioridad | P1 |
| Estimación | 4 h |
| Depende de | ENV-T039, ENV-T019, ENV-T045, ENV-T037 |
| Bloquea a | ENV-T050, ENV-T053, ENV-T072 |
| Sprint propuesto | S09 (2027-01-18 → 2027-01-22) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/window.rs, src/ui/editor.rs |

**Descripción.** Acciones Ctrl+Z / Ctrl+Shift+Z conectadas al EditHistory según D-13, sensibilidad de las acciones, refresco de vista conservando foco y recálculo de dirty.

**Criterios de aceptación**
- [ ] Undo/redo funciona para editar valor, crear y eliminar.
- [ ] Deshacer hasta el estado guardado vuelve a Sin cambios; deshacer tras guardar marca Modificado.

**Pruebas:** Prueba manual de teclado con foco en campo y fuera de él.

<a id="env-t047"></a>
#### ENV-T047 — Implementar la búsqueda de variables por clave

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-007; PRD §7.3, §7.4 |
| Prioridad | P1 |
| Estimación | 4 h |
| Depende de | ENV-T036 |
| Bloquea a | ENV-T053, ENV-T072 |
| Sprint propuesto | S08 (2027-01-11 → 2027-01-15) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/editor.rs |

**Descripción.** GtkSearchBar con Ctrl+F y Esc; filtra ocurrencias por nombre de clave con contador; nunca busca en valores ni indexa fuera de memoria.

**Criterios de aceptación**
- [ ] Buscar DATABASE filtra las filas cuya clave lo contiene y muestra el contador.
- [ ] Un término presente solo en un valor no produce coincidencias.

**Pruebas:** Test unitario del filtro; prueba manual.

<a id="env-t048"></a>
#### ENV-T048 — Implementar la ocultación visual de secretos con revelado por fila

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-012; PRD US-04, §8.3; decisión D-17 |
| Prioridad | P1 |
| Estimación | 5 h |
| Depende de | ENV-T036, ENV-T022, ENV-T038 |
| Bloquea a | ENV-T053 |
| Sprint propuesto | S09 (2027-01-18 → 2027-01-22) |
| Estado | Backlog |
| Agentes | Implementador, QA, Security Reviewer |
| Entregables / archivos | src/ui/assignment_row.rs |

**Descripción.** Filas clasificadas por la heurística se muestran ocultas por defecto; conmutador revelar/ocultar por fila (alcance según D-17); los valores multilínea ocultos muestran un marcador hasta revelarse; etiquetas accesibles sin leer el valor.

**Criterios de aceptación**
- [ ] DATABASE_PASSWORD aparece oculto al abrir; DEBUG no.
- [ ] El lector de pantalla anuncia Valor oculto, no el contenido.
- [ ] La interfaz no promete cifrado ni borrado de memoria.

**Pruebas:** Prueba manual con Orca; test unitario de la clasificación ya en T022.

<a id="env-t049"></a>
#### ENV-T049 — Implementar copiar valor y copiar documento

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-011; PRD US-04 |
| Prioridad | P1 |
| Estimación | 3 h |
| Depende de | ENV-T036, ENV-T014 |
| Bloquea a | ENV-T053 |
| Sprint propuesto | S09 (2027-01-18 → 2027-01-22) |
| Estado | Backlog |
| Agentes | Implementador, QA, Security Reviewer |
| Entregables / archivos | src/ui/assignment_row.rs, src/ui/window.rs |

**Descripción.** Copiar valor copia el valor editable actual decodificado; Copiar documento copia la serialización actual con cambios sin guardar, comentarios y Raw; solo por acción explícita, con aviso de que el portapapeles puede conservar secretos.

**Criterios de aceptación**
- [ ] Copiar documento con cambios sin guardar devuelve exactamente serialize() en memoria.
- [ ] No existe ninguna copia automática.

**Pruebas:** Test que compara portapapeles con serialize (GTK con display); prueba manual.

<a id="env-t050"></a>
#### ENV-T050 — Implementar el diálogo de pegado con previsualización

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-010; PRD US-03; TD §6.3, §10.3 |
| Prioridad | P1 |
| Estimación | 5 h |
| Depende de | ENV-T021, ENV-T046 |
| Bloquea a | ENV-T051 |
| Sprint propuesto | S09 (2027-01-18 → 2027-01-22) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/paste_dialog.rs |

**Descripción.** Acción Pegar asignaciones que lee el texto del portapapeles de forma asíncrona y muestra entradas, claves, recuentos y advertencias con valores ocultos; Aplicar genera un único Batch; Cancelar no cambia nada.

**Criterios de aceptación**
- [ ] La previsualización no muestra valores en claro por defecto.
- [ ] Aplicar se deshace con un único Ctrl+Z.

**Pruebas:** Prueba manual con bloques válidos, multilínea y mixtos.

<a id="env-t051"></a>
#### ENV-T051 — Implementar la resolución de duplicados y líneas inválidas en el pegado

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-010; PRD US-03 |
| Prioridad | P1 |
| Estimación | 4 h |
| Depende de | ENV-T050 |
| Bloquea a | ENV-T053 |
| Sprint propuesto | S09 (2027-01-18 → 2027-01-22) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/paste_dialog.rs |

**Descripción.** Por cada clave existente: Añadir, Reemplazar ocurrencia elegida (selección explícita si hay varias) o Cancelar; las líneas no interpretables se listan con su número; Insertar solo válidas requiere confirmación explícita.

**Criterios de aceptación**
- [ ] Con varias ocurrencias, el reemplazo exige elegir cuál.
- [ ] Nunca se aplica un pegado parcial sin confirmación.

**Pruebas:** Prueba manual; tests del PastePlan ya en T021.

<a id="env-t052"></a>
#### ENV-T052 — Implementar la pantalla de bienvenida y la acción Abrir archivo

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-006; PRD §7.1, §7.4 (Ctrl+O); TD §10.3 |
| Prioridad | P1 |
| Estimación | 3 h |
| Depende de | ENV-T034 |
| Bloquea a | ENV-T053, ENV-T061, ENV-T072 |
| Sprint propuesto | S09 (2027-01-18 → 2027-01-22) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/window.rs |

**Descripción.** AdwStatusPage de bienvenida con Abrir archivo (Agregar proyecto se conecta en T061) y Ctrl+O con GtkFileDialog.

**Criterios de aceptación**
- [ ] Ctrl+O abre el selector y el archivo elegido se carga en Quick Edit.
- [ ] Abrir un archivo no lo registra como proyecto.

**Pruebas:** Prueba manual.

<a id="env-t053"></a>
#### ENV-T053 — Integrar Quick Edit de extremo a extremo y corregir defectos

| Campo | Valor |
|---|---|
| Épica | E5 Quick Edit |
| Origen (requisito/decisión) | ENV-006 a ENV-013; PRD §3 Situaciones A y D; hito PRD 2 |
| Prioridad | P1 |
| Estimación | 6 h |
| Depende de | ENV-T042, ENV-T044, ENV-T046, ENV-T047, ENV-T048, ENV-T049, ENV-T051, ENV-T052, ENV-T043, ENV-T041, ENV-T038 |
| Bloquea a | ENV-T071, ENV-T074, ENV-T080 |
| Sprint propuesto | S10 (2027-01-25 → 2027-01-29) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | Correcciones en src/; registro de defectos |

**Descripción.** Tiempo reservado para integración: ejecutar las Situaciones A y D en nativo, corregir defectos encontrados y dejar las pruebas en verde.

**Criterios de aceptación**
- [ ] Situación A completada sin registrar proyecto.
- [ ] Situación D: edición externa con y sin cambios locales sin pérdida.
- [ ] Defectos P0 abiertos = 0; P1 documentados con plan.

**Pruebas:** Suite automática completa + guion manual parcial.

### E6 — Workspace

<a id="env-t054"></a>
#### ENV-T054 — Spike: acceso a carpetas, persistencia de permisos y CLI en Flatpak

| Campo | Valor |
|---|---|
| Épica | E6 Workspace |
| Origen (requisito/decisión) | PRD §8.9, §9, §13, US-06; TD §9, §12, §16; decisión D-19 |
| Prioridad | P0 |
| Estimación | 8 h — incertidumbre alta |
| Depende de | ENV-T001, ENV-T025 |
| Bloquea a | ENV-T055, ENV-T077 |
| Sprint propuesto | S03 (2026-12-07 → 2026-12-11) |
| Estado | Backlog |
| Agentes | Explorador, Implementador, Security Reviewer |
| Entregables / archivos | Matriz de resultados y propuesta de política de permisos en DECISIONS.md (D-19); prototipo desechable |

**Descripción.** Spike con timebox de 8 h sobre un prototipo mínimo empaquetado: seleccionar una carpeta con GtkFileDialog vía portal sin --filesystem=home, enumerarla, leer/escribir un dotenv con replace_contents, monitorizarlo y recuperar acceso tras reiniciar la app usando la referencia almacenada; comprobar también el paso de rutas por flatpak run con reenvío de archivos.

**Criterios de aceptación**
- [ ] Resultado documentado para enumerar, escribir, monitorizar y persistir tras reinicio.
- [ ] Propuesta de finish-args mínimos con justificación.
- [ ] Criterio de salida: Workspace viable en Flatpak con portales, o alternativa explícita para decisión del usuario.

**Pruebas:** Ejecución manual en sesión GNOME real con evidencia por caso.

**Riesgos / seguridad:** Si los portales no permiten persistencia, Workspace en Flatpak requiere permisos ampliados o limitaciones comunicadas (bloquea release Flatpak según TD §16).

<a id="env-t055"></a>
#### ENV-T055 — Implementar el catálogo de proyectos sin secretos

| Campo | Valor |
|---|---|
| Épica | E6 Workspace |
| Origen (requisito/decisión) | ENV-014, ENV-015; PRD §4, US-06; TD ADR-007, §11.2; decisiones D-10, D-19 |
| Prioridad | P2 |
| Estimación | 6 h |
| Depende de | ENV-T054, ENV-T004 |
| Bloquea a | ENV-T056, ENV-T057, ENV-T064 |
| Sprint propuesto | S10 (2027-01-25 → 2027-01-29) |
| Estado | Backlog |
| Agentes | Implementador, QA, Security Reviewer |
| Entregables / archivos | src/domain/project.rs, src/infrastructure/project_store.rs |

**Descripción.** Modelo Project/Service con id, nombre visual, URI, etiquetas por archivo y última vista; TOML con schema_version=1 en $XDG_CONFIG_HOME/envy/projects.toml (o ruta del sandbox); validación al cargar, deduplicación por URI, versión desconocida => no sobrescribir; escritura segura; nunca valores.

**Criterios de aceptación**
- [ ] El archivo de catálogo nunca contiene valores de variables (test con centinela).
- [ ] Un catálogo corrupto o de versión futura no se sobrescribe y se informa.
- [ ] Los proyectos se restauran tras reiniciar.

**Pruebas:** Tests unitarios de serialización/validación; test de integración con tempdir.

<a id="env-t056"></a>
#### ENV-T056 — Implementar la gestión de proyectos y carpetas de servicio

| Campo | Valor |
|---|---|
| Épica | E6 Workspace |
| Origen (requisito/decisión) | ENV-014, ENV-015; PRD US-06 |
| Prioridad | P2 |
| Estimación | 6 h |
| Depende de | ENV-T055, ENV-T033 |
| Bloquea a | ENV-T059 |
| Sprint propuesto | S10 (2027-01-25 → 2027-01-29) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/services/project_service.rs, src/ui/workspace.rs |

**Descripción.** Agregar proyecto con selector de carpeta, renombrar nombre visual, quitar referencia con confirmación que aclara que no se borran archivos; agregar, renombrar y quitar carpetas de servicio de un proyecto.

**Criterios de aceptación**
- [ ] Quitar un proyecto no toca su carpeta ni sus archivos (verificado con listado antes/después).
- [ ] El nombre de servicio deriva de la carpeta y es renombrable.

**Pruebas:** Tests del servicio; prueba manual de diálogos.

<a id="env-t057"></a>
#### ENV-T057 — Implementar el descubrimiento de archivos dotenv en carpetas registradas

| Campo | Valor |
|---|---|
| Épica | E6 Workspace |
| Origen (requisito/decisión) | ENV-016; PRD US-06; TD §11.1; decisión D-29 |
| Prioridad | P2 |
| Estimación | 5 h |
| Depende de | ENV-T055, ENV-T027 |
| Bloquea a | ENV-T058, ENV-T059 |
| Sprint propuesto | S10 (2027-01-25 → 2027-01-29) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/services/environment_service.rs, src/infrastructure/filesystem.rs |

**Descripción.** Enumera solo hijos inmediatos de la raíz y de cada servicio; patrones exactos .env, .env.*, *.env con la regla aprobada para casos como .env.example.txt; excluye no regulares y marca enlaces simbólicos con aviso; orden .env primero y luego orden natural; no lee contenido.

**Criterios de aceptación**
- [ ] .envrc no aparece; node_modules/.env no se recorre; un enlace simbólico aparece con aviso y sin edición automática.
- [ ] No se abre el contenido de ningún archivo durante el listado.

**Pruebas:** Tests de integración con tempdir que incluyen negativos.

<a id="env-t058"></a>
#### ENV-T058 — Implementar el refresco del descubrimiento por monitor y acción Refrescar

| Campo | Valor |
|---|---|
| Épica | E6 Workspace |
| Origen (requisito/decisión) | ENV-016; TD §11.1 paso 6 |
| Prioridad | P2 |
| Estimación | 3 h |
| Depende de | ENV-T057 |
| Bloquea a | ENV-T063 |
| Sprint propuesto | S11 (2027-02-01 → 2027-02-05) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/services/environment_service.rs, src/ui/workspace.rs |

**Descripción.** Monitores de directorio en carpetas registradas cuando estén disponibles y acción manual Refrescar; actualiza la lista sin perder sesiones abiertas.

**Criterios de aceptación**
- [ ] Crear un .env.local en la carpeta lo muestra tras evento o Refrescar.
- [ ] Las sesiones abiertas con cambios no se ven afectadas.

**Pruebas:** Test de integración con tempdir; prueba manual.

<a id="env-t059"></a>
#### ENV-T059 — Implementar la barra lateral del Workspace con AdwNavigationSplitView

| Campo | Valor |
|---|---|
| Épica | E6 Workspace |
| Origen (requisito/decisión) | ENV-014, ENV-015, ENV-016; PRD §7.2; TD §10.1-10.2 |
| Prioridad | P2 |
| Estimación | 6 h |
| Depende de | ENV-T056, ENV-T057 |
| Bloquea a | ENV-T060, ENV-T061, ENV-T062, ENV-T064 |
| Sprint propuesto | S11 (2027-02-01 → 2027-02-05) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/workspace.rs, resources/ui/workspace.ui |

**Descripción.** Panel lateral con proyectos, servicios y archivos en filas nativas, indicador Modificado por archivo y zona de contenido que aloja el EditorView compartido.

**Criterios de aceptación**
- [ ] La jerarquía proyecto > servicio > archivo es navegable con teclado.
- [ ] El indicador Modificado es visible en la fila del archivo.

**Pruebas:** Prueba manual; test GTK si hay display.

<a id="env-t060"></a>
#### ENV-T060 — Implementar la selección de archivo y la conmutación de sesiones

| Campo | Valor |
|---|---|
| Épica | E6 Workspace |
| Origen (requisito/decisión) | ENV-014; PRD §7.2; TD ADR-005, §9 |
| Prioridad | P2 |
| Estimación | 5 h |
| Depende de | ENV-T059, ENV-T036, ENV-T035 |
| Bloquea a | ENV-T063, ENV-T065, ENV-T082 |
| Sprint propuesto | S11 (2027-02-01 → 2027-02-05) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/workspace.rs, src/services/document_service.rs |

**Descripción.** Seleccionar un archivo abre o reutiliza su DocumentSession por URI, mantiene en memoria las sesiones modificadas al cambiar de archivo y es coherente con ventanas de Quick Edit que tengan el mismo archivo.

**Criterios de aceptación**
- [ ] Cambiar de archivo con cambios sin guardar no los pierde.
- [ ] El mismo archivo no tiene dos sesiones divergentes.

**Pruebas:** Prueba manual; tests del registro de sesiones.

<a id="env-t061"></a>
#### ENV-T061 — Implementar estados vacíos y restauración de la última vista

| Campo | Valor |
|---|---|
| Épica | E6 Workspace |
| Origen (requisito/decisión) | ENV-014; PRD US-01, US-06, §7.1; TD §10.3 |
| Prioridad | P2 |
| Estimación | 4 h |
| Depende de | ENV-T059, ENV-T052 |
| Bloquea a | ENV-T063, ENV-T082 |
| Sprint propuesto | S11 (2027-02-01 → 2027-02-05) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/workspace.rs, src/ui/window.rs |

**Descripción.** Bienvenida sin proyectos con Agregar proyecto conectado; proyecto sin archivos indica dónde se buscó y ofrece agregar servicio o abrir otro archivo; sin argumentos se restaura la última vista accesible; un proyecto con acceso revocado muestra un estado recuperable.

**Criterios de aceptación**
- [ ] Sin proyectos se muestra la bienvenida con ambas acciones.
- [ ] Reiniciar la app restaura el último proyecto/archivo si sigue accesible.

**Pruebas:** Prueba manual de cada estado.

<a id="env-t062"></a>
#### ENV-T062 — Implementar la navegación adaptable en ventana estrecha

| Campo | Valor |
|---|---|
| Épica | E6 Workspace |
| Origen (requisito/decisión) | ENV-020; PRD §7.2; TD §10.1 |
| Prioridad | P1 |
| Estimación | 4 h |
| Depende de | ENV-T059 |
| Bloquea a | ENV-T063, ENV-T073 |
| Sprint propuesto | S11 (2027-02-01 → 2027-02-05) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/workspace.rs, resources/ui/*.ui |

**Descripción.** Breakpoints de libadwaita: split view colapsado con navegación de ida y vuelta, acciones de cabecera reorganizadas y filas utilizables en ancho de móvil.

**Criterios de aceptación**
- [ ] A 360 px de ancho se completa el flujo elegir archivo > editar > guardar > volver.
- [ ] No hay contenido recortado ni inaccesible por teclado.

**Pruebas:** Prueba manual redimensionando; captura como evidencia.

<a id="env-t063"></a>
#### ENV-T063 — Integrar Workspace de extremo a extremo y corregir defectos

| Campo | Valor |
|---|---|
| Épica | E6 Workspace |
| Origen (requisito/decisión) | ENV-014 a ENV-019; PRD §3 Situaciones B, C y E; hito PRD 3 |
| Prioridad | P2 |
| Estimación | 6 h |
| Depende de | ENV-T058, ENV-T060, ENV-T061, ENV-T062, ENV-T064, ENV-T065, ENV-T066 |
| Bloquea a | ENV-T075, ENV-T080 |
| Sprint propuesto | S12 (2027-02-08 → 2027-02-12) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | Correcciones en src/; registro de defectos |

**Descripción.** Tiempo reservado para integración: ejecutar las Situaciones B, C y E en nativo, corregir defectos y dejar la suite en verde.

**Criterios de aceptación**
- [ ] Situaciones B, C y E completadas sin navegar manualmente por el sistema de archivos.
- [ ] Defectos P0 abiertos = 0.

**Pruebas:** Suite automática completa + guion manual de Workspace.

### E7 — Environment Tools

<a id="env-t064"></a>
#### ENV-T064 — Implementar las etiquetas de entorno visibles y corregibles

| Campo | Valor |
|---|---|
| Épica | E7 Environment Tools |
| Origen (requisito/decisión) | ENV-017; PRD US-07; TD §10.4 |
| Prioridad | P2 |
| Estimación | 5 h |
| Depende de | ENV-T024, ENV-T059, ENV-T055 |
| Bloquea a | ENV-T063, ENV-T073 |
| Sprint propuesto | S12 (2027-02-08 → 2027-02-12) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/workspace.rs, src/ui/editor.rs, src/infrastructure/project_store.rs |

**Descripción.** Etiqueta con texto + icono + color mediante clases de estilo/tokens de libadwaita (no colores fijos), inferida por T024, corregible desde un menú y persistida en el catálogo por URI de archivo.

**Criterios de aceptación**
- [ ] Production es legible sin color (texto visible) en claro, oscuro y alto contraste.
- [ ] La corrección manual persiste tras reiniciar.

**Pruebas:** Prueba manual en los tres modos de contraste; test de persistencia.

<a id="env-t065"></a>
#### ENV-T065 — Implementar la comparación de claves con .env.example e inserción de faltantes

| Campo | Valor |
|---|---|
| Épica | E7 Environment Tools |
| Origen (requisito/decisión) | ENV-018; PRD US-07, Situación E; decisión D-18 |
| Prioridad | P2 |
| Estimación | 6 h |
| Depende de | ENV-T023, ENV-T045, ENV-T060 |
| Bloquea a | ENV-T063, ENV-T073 |
| Sprint propuesto | S12 (2027-02-08 → 2027-02-12) |
| Estado | Backlog |
| Agentes | Implementador, QA, Security Reviewer |
| Entregables / archivos | src/ui/editor.rs (panel de comparación), src/services/environment_service.rs |

**Descripción.** Si existe un .env.example legible en la misma carpeta, panel con Faltan / Solo en archivo actual / En ambos sin valores; pulsar una clave faltante prepara un InsertAssignment con valor vacío tras confirmación; disponibilidad en Quick Edit según D-18.

**Criterios de aceptación**
- [ ] Nunca se copian valores del .env.example.
- [ ] La inserción requiere confirmación y es deshacible.

**Pruebas:** Prueba manual con fixtures de ejemplo; tests de T023.

<a id="env-t066"></a>
#### ENV-T066 — Implementar los avisos de claves duplicadas en el editor

| Campo | Valor |
|---|---|
| Épica | E7 Environment Tools |
| Origen (requisito/decisión) | ENV-019; PRD US-07 |
| Prioridad | P2 |
| Estimación | 3 h |
| Depende de | ENV-T020, ENV-T036 |
| Bloquea a | ENV-T063 |
| Sprint propuesto | S11 (2027-02-01 → 2027-02-05) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/ui/assignment_row.rs, src/ui/editor.rs |

**Descripción.** Distintivo Duplicada ×n en cada ocurrencia (texto, no solo color) y resumen en el editor; sin indicar cuál prevalece.

**Criterios de aceptación**
- [ ] duplicate-keys.env muestra el recuento en cada ocurrencia.
- [ ] El aviso es accesible por lector de pantalla.

**Pruebas:** Prueba manual.

### E8 — Quality Assurance

<a id="env-t067"></a>
#### ENV-T067 — Crear la matriz de pruebas de integración de guardado seguro

| Campo | Valor |
|---|---|
| Épica | E8 Quality Assurance |
| Origen (requisito/decisión) | ENV-004; PRD US-05, §8.5, §8.7-8.8; TD §8.2, §14.2 |
| Prioridad | P0 |
| Estimación | 6 h |
| Depende de | ENV-T028 |
| Bloquea a | ENV-T075 |
| Sprint propuesto | S05 (2026-12-21 → 2026-12-25) |
| Estado | Backlog |
| Agentes | QA, Security Reviewer |
| Entregables / archivos | tests/file_conflicts.rs (o tests/save_safety.rs) |

**Descripción.** Tests con tempfile: modo 0600 y 0644, archivo sin escritura, directorio sin permiso de creación, enlace simbólico, cambio externo justo antes de guardar, disco cambiado sin evento, archivo eliminado. Verifican ausencia de truncado, de sobrescritura silenciosa y de respaldos.

**Criterios de aceptación**
- [ ] Cada fila de la matriz tiene un test automatizado o una justificación de prueba manual.
- [ ] Ningún caso termina con éxito declarado y contenido distinto en disco.

**Pruebas:** La propia suite; ejecutada en CI.

**Filas añadidas por la revisión de seguridad del spike ENV-T025 (2026-11-22):** fallo de escritura a mitad (`setrlimit(RLIMIT_FSIZE)` con `SIGXFSZ` ignorado) y stream soltado sin cerrar; archivo borrado entre pre-vuelo y guardado; symlink o archivo cambiado tras el pre-vuelo (seam); directorio de grupo con y sin setgid; uid o gid ajeno (contenedor); directorio con ACL por defecto y etiqueta SELinux propia; `chattr +i`/`+a` (manual); directorio sticky; Guardar como sobre un symlink colgante; umask 000/002/022; temporal creado en 0600 (seam o shim).

<a id="env-t068"></a>
#### ENV-T068 — Crear la suite de regresión de edición

| Campo | Valor |
|---|---|
| Épica | E8 Quality Assurance |
| Origen (requisito/decisión) | ENV-002, ENV-008, ENV-009, ENV-010; PRD §14; TD §14.1 |
| Prioridad | P0 |
| Estimación | 5 h |
| Depende de | ENV-T019, ENV-T021 |
| Bloquea a | Ninguna |
| Sprint propuesto | S06 (2026-12-28 → 2027-01-01) |
| Estado | Backlog |
| Agentes | QA |
| Entregables / archivos | tests/editing.rs |

**Descripción.** tests/editing.rs: SetValue en cada estilo y multilínea con Unicode, insertar/eliminar con duplicados, lote de pegado + undo = bytes originales, redo = estado del lote, undo tras guardar => dirty, rechazos del codec.

**Criterios de aceptación**
- [ ] Todas las pruebas mínimas de TD §14.1 sobre edición están automatizadas.
- [ ] Los fallos no imprimen valores de fixtures.

**Pruebas:** La propia suite; ejecutada en CI.

<a id="env-t069"></a>
#### ENV-T069 — Crear las pruebas de integración de conflictos y monitorización

| Campo | Valor |
|---|---|
| Épica | E8 Quality Assurance |
| Origen (requisito/decisión) | ENV-005; PRD US-05, §10 (cero pérdidas); TD §7.2, §14.2 |
| Prioridad | P0 |
| Estimación | 6 h |
| Depende de | ENV-T030, ENV-T031 |
| Bloquea a | ENV-T075 |
| Sprint propuesto | S06 (2026-12-28 → 2027-01-01) |
| Estado | Backlog |
| Agentes | QA, Security Reviewer |
| Entregables / archivos | tests/file_conflicts.rs |

**Descripción.** Limpio + cambio externo => recarga; modificado + cambio externo => conflicto con buffer intacto; eventos repetidos => una recarga; escritor externo por renombrado y por truncado+escritura; borrado/movimiento durante edición; guardado propio no marcado; guardado tras evento perdido => conflicto.

**Criterios de aceptación**
- [ ] Ningún test permite descartar ediciones locales o externas sin acción expresa.
- [ ] Las pruebas son deterministas (sin dependencias de temporización frágiles).

**Pruebas:** La propia suite; ejecutada en CI sin display.

<a id="env-t070"></a>
#### ENV-T070 — Auditar y probar la ausencia de secretos en logs y errores del núcleo

| Campo | Valor |
|---|---|
| Épica | E8 Quality Assurance |
| Origen (requisito/decisión) | PRD §8.2, §10 (privacidad de logs); TD §13 |
| Prioridad | P0 |
| Estimación | 4 h |
| Depende de | ENV-T030, ENV-T021 |
| Bloquea a | Ninguna |
| Sprint propuesto | S06 (2026-12-28 → 2027-01-01) |
| Estado | Backlog |
| Agentes | QA, Security Reviewer |
| Entregables / archivos | tests/secrets_leak.rs, informe en la tarea |

**Descripción.** Con fixtures que contienen el centinela, ejercita parse, edición, pegado, guardado y errores con un subscriber de tracing a nivel TRACE capturando la salida; verifica que el centinela no aparece en logs, Display/Debug de errores ni mensajes de panic; revisión de todas las llamadas a macros de tracing.

**Criterios de aceptación**
- [ ] El centinela no aparece en ninguna salida capturada.
- [ ] Lista de llamadas a tracing revisadas adjunta como evidencia.

**Pruebas:** La propia suite; repetida con UI en T075.

**Heredado de la revisión de seguridad de ENV-T004 (2026-11-21):** añadir un chequeo automático (lint `disallowed-macros`/`disallowed-methods` o grep en CI) contra `?value`/`%value` en `tracing`; verificar que las rutas absolutas no llegan a analítica, preferencias exportables ni informes de fallo.

<a id="env-t071"></a>
#### ENV-T071 — Redactar el guion de pruebas manuales de UI y sistema

| Campo | Valor |
|---|---|
| Épica | E8 Quality Assurance |
| Origen (requisito/decisión) | PRD §10, §14; TD §14.3 |
| Prioridad | P1 |
| Estimación | 4 h |
| Depende de | ENV-T053 |
| Bloquea a | Ninguna |
| Sprint propuesto | S10 (2027-01-25 → 2027-01-29) |
| Estado | Backlog |
| Agentes | QA |
| Entregables / archivos | docs/testing/MANUAL_TESTS.md |

**Descripción.** docs/testing/MANUAL_TESTS.md con casos de teclado, foco, política de Ctrl+Z, conflictos con editores externos, enlace simbólico, permisos, solo lectura, UTF-8 inválido, aviso de portapapeles, ventana estrecha, oscuro/alto contraste y huecos para Flatpak (T082). Cada caso: pasos, resultado esperado y evidencia.

**Criterios de aceptación**
- [ ] Cada criterio de aceptación de UI del PRD está cubierto por al menos un caso.
- [ ] Los casos son ejecutables por otra persona sin contexto adicional.

**Pruebas:** Ejecución piloto de 3 casos.

<a id="env-t072"></a>
#### ENV-T072 — Consolidar y verificar los atajos de teclado

| Campo | Valor |
|---|---|
| Épica | E8 Quality Assurance |
| Origen (requisito/decisión) | ENV-020; PRD §7.4 |
| Prioridad | P1 |
| Estimación | 3 h |
| Depende de | ENV-T046, ENV-T047, ENV-T052, ENV-T045 |
| Bloquea a | ENV-T073 |
| Sprint propuesto | S11 (2027-02-01 → 2027-02-05) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | src/application.rs, src/ui/window.rs |

**Descripción.** Todos los atajos de PRD §7.4 registrados como GAction con aceleradores; verificación de conflictos con campos de texto (Ctrl+Z, Ctrl+C) y comportamiento de Esc.

**Criterios de aceptación**
- [ ] Cada atajo de PRD §7.4 funciona con foco en lista y en campo, según la política D-13.
- [ ] Ctrl+C en un campo copia la selección del campo, no el documento.

**Pruebas:** Guion manual de atajos.

<a id="env-t073"></a>
#### ENV-T073 — Auditar accesibilidad y HIG y corregir hallazgos

| Campo | Valor |
|---|---|
| Épica | E8 Quality Assurance |
| Origen (requisito/decisión) | ENV-020; PRD §10 (accesibilidad), §14; TD §14.3 |
| Prioridad | P1 |
| Estimación | 6 h |
| Depende de | ENV-T062, ENV-T064, ENV-T065, ENV-T072 |
| Bloquea a | ENV-T075 |
| Sprint propuesto | S12 (2027-02-08 → 2027-02-12) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | Correcciones en src/ y resources/; informe de auditoría en la tarea |

**Descripción.** Orca en los flujos principales, finalización con solo teclado de todas las acciones esenciales, foco visible, nombres y descripciones accesibles en botones de icono y valores ocultos, oscuro y alto contraste, color siempre acompañado de texto. Se corrigen hallazgos dentro del presupuesto y el resto se registra como defectos.

**Criterios de aceptación**
- [ ] Todas las acciones esenciales se completan con teclado.
- [ ] Ningún estado o etiqueta depende solo del color.

**Pruebas:** Guion manual con Orca; capturas en alto contraste.

<a id="env-t074"></a>
#### ENV-T074 — Medir la línea base de rendimiento y la métrica principal

| Campo | Valor |
|---|---|
| Épica | E8 Quality Assurance |
| Origen (requisito/decisión) | PRD §10 (métrica principal; objetivos tras línea base) |
| Prioridad | P1 |
| Estimación | 3 h |
| Depende de | ENV-T053 |
| Bloquea a | Ninguna |
| Sprint propuesto | S11 (2027-02-01 → 2027-02-05) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | Resultados en la tarea y en docs/testing/ |

**Descripción.** Cronometrar abrir-localizar-guardar en hardware de referencia y medir carga/render con archivos sintéticos de 1 000 y 5 000 líneas; documentar resultados y proponer objetivos numéricos.

**Criterios de aceptación**
- [ ] Mediciones reproducibles con hardware y versión anotados.
- [ ] Objetivos propuestos para aprobación, sin prometer cifras no medidas.

**Pruebas:** Medición manual y script de generación de archivos sintéticos (sin secretos reales).

<a id="env-t075"></a>
#### ENV-T075 — Endurecer en nativo con regresión completa y corrección de defectos

| Campo | Valor |
|---|---|
| Épica | E8 Quality Assurance |
| Origen (requisito/decisión) | PRD §11 hito 4, §14; criterio de salida PRD §11 |
| Prioridad | P0 |
| Estimación | 6 h |
| Depende de | ENV-T073, ENV-T063, ENV-T069, ENV-T067 |
| Bloquea a | ENV-T076 |
| Sprint propuesto | S14 (2027-02-22 → 2027-02-26) |
| Estado | Backlog |
| Agentes | QA, Security Reviewer |
| Entregables / archivos | Informe de regresión; correcciones |

**Descripción.** Regresión completa automática y manual en la distribución de prueba oficial (D-06), repetición de la auditoría de fugas incluyendo UI y tracing, y corrección de defectos.

**Criterios de aceptación**
- [ ] Suite automática en verde; guion manual ejecutado con evidencia.
- [ ] Auditoría de fugas con UI sin hallazgos.
- [ ] Defectos P0 = 0.

**Pruebas:** Suite completa + guion manual.

<a id="env-t076"></a>
#### ENV-T076 — Ejecutar la regresión del RC y preparar la evaluación go/no-go

| Campo | Valor |
|---|---|
| Épica | E8 Quality Assurance |
| Origen (requisito/decisión) | PRD §11 criterio de salida, §14 |
| Prioridad | P0 |
| Estimación | 5 h |
| Depende de | ENV-T085, ENV-T075, ENV-T082, ENV-T086 |
| Bloquea a | ENV-T087 |
| Sprint propuesto | S15 (2027-03-01 → 2027-03-05) |
| Estado | Backlog |
| Agentes | QA, Security Reviewer |
| Entregables / archivos | Informe go/no-go |

**Descripción.** Suite completa y guion manual sobre el build RC en nativo y Flatpak; comprobación de criterios de release de DEFINITION_OF_DONE.md; informe go/no-go para el usuario.

**Criterios de aceptación**
- [ ] Cada criterio de release tiene evidencia o se declara no cumplido.
- [ ] La decisión go/no-go queda en manos del usuario.

**Pruebas:** Suite completa + guion manual en nativo y Flatpak.

### E9 — Distribution & Release

<a id="env-t077"></a>
#### ENV-T077 — Crear el manifiesto Flatpak con dependencias Cargo offline

| Campo | Valor |
|---|---|
| Épica | E9 Distribution & Release |
| Origen (requisito/decisión) | PRD §8.9, §9; TD §12; decisiones D-06, D-19 |
| Prioridad | P0 |
| Estimación | 6 h |
| Depende de | ENV-T054, ENV-T033 |
| Bloquea a | ENV-T081, ENV-T082, ENV-T084 |
| Sprint propuesto | S12 (2027-02-08 → 2027-02-12) |
| Estado | Backlog |
| Agentes | Implementador, Security Reviewer |
| Entregables / archivos | packaging/flatpak/`APP_ID`.json o .yml, cargo-sources.json |

**Descripción.** Manifiesto con runtime GNOME de D-06, extensión rust-stable, fuentes Cargo generadas para build offline y finish-args mínimos según D-19 (sin --filesystem=home silencioso); build y ejecución local.

**Criterios de aceptación**
- [ ] flatpak-builder construye y la app se ejecuta en el sandbox.
- [ ] Cada permiso de finish-args está justificado por escrito.

**Pruebas:** Build local; arranque y Quick Edit básico dentro del sandbox.

<a id="env-t078"></a>
#### ENV-T078 — Crear el desktop entry, los metadatos AppStream y el esquema de ajustes

| Campo | Valor |
|---|---|
| Épica | E9 Distribution & Release |
| Origen (requisito/decisión) | PRD §14 (metadatos); TD §4 data/; decisiones D-07, D-10 |
| Prioridad | P1 |
| Estimación | 5 h |
| Depende de | ENV-T033 |
| Bloquea a | ENV-T081, ENV-T084 |
| Sprint propuesto | S13 (2027-02-15 → 2027-02-19) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | data/`APP_ID`.desktop, data/`APP_ID`.metainfo.xml, data/`APP_ID`.gschema.xml (si aplica) |

**Descripción.** Archivo .desktop, metainfo AppStream (descripción, release 0.1.0, OARS, licencia, capturas) y gschema si D-10 lo requiere; validados con desktop-file-validate y appstreamcli.

**Criterios de aceptación**
- [ ] desktop-file-validate y appstreamcli validate sin errores.
- [ ] El app ID coincide en todos los archivos (requiere D-07 cerrada).

**Pruebas:** Validadores en CI o local.

<a id="env-t079"></a>
#### ENV-T079 — Diseñar el icono de la aplicación y su variante simbólica

| Campo | Valor |
|---|---|
| Épica | E9 Distribution & Release |
| Origen (requisito/decisión) | PRD §14; TD §15 Hito 4; GNOME HIG |
| Prioridad | P1 |
| Estimación | 5 h — incertidumbre media (diseño gráfico) |
| Depende de | Ninguna |
| Bloquea a | ENV-T084 |
| Sprint propuesto | S13 (2027-02-15 → 2027-02-19) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | data/icons/ |

**Descripción.** Icono SVG escalable y simbólico conforme a la HIG, revisado en tamaños pequeños y grandes y en claro/oscuro.

**Criterios de aceptación**
- [ ] El icono es legible a 16 px y 128 px.
- [ ] Cumple los requisitos de icono de Flathub.

**Pruebas:** Revisión visual con capturas.

**Riesgos / seguridad:** Requiere habilidad de diseño; alternativa: icono geométrico simple.

<a id="env-t080"></a>
#### ENV-T080 — Preparar la internacionalización con gettext

| Campo | Valor |
|---|---|
| Épica | E9 Distribution & Release |
| Origen (requisito/decisión) | PRD §11 hito 4 (traducciones preparadas); decisión D-21 |
| Prioridad | P1 |
| Estimación | 6 h |
| Depende de | ENV-T063, ENV-T053 |
| Bloquea a | Ninguna |
| Sprint propuesto | S13 (2027-02-15 → 2027-02-19) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | po/, src/i18n.rs (o equivalente), build |

**Descripción.** Integración de gettext (dependencia sujeta a aprobación), sustitución del helper identidad, POTFILES, generación del .pot y verificación de que no quedan cadenas visibles sin marcar; idiomas incluidos según D-21.

**Criterios de aceptación**
- [ ] Se genera un .pot con todas las cadenas visibles.
- [ ] Una traducción de prueba cambia la UI al cambiar de locale.

**Pruebas:** Prueba manual con LANGUAGE de prueba; búsqueda de cadenas sin marcar.

<a id="env-t081"></a>
#### ENV-T081 — Configurar la asociación de archivos y documentar la CLI en Flatpak

| Campo | Valor |
|---|---|
| Épica | E9 Distribution & Release |
| Origen (requisito/decisión) | PRD US-01, §7.1; TD §9; decisión D-20 |
| Prioridad | P1 |
| Estimación | 5 h |
| Depende de | ENV-T077, ENV-T034, ENV-T078 |
| Bloquea a | ENV-T083 |
| Sprint propuesto | S13 (2027-02-15 → 2027-02-19) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | data/*.desktop, data/mime (si aplica), README (sección CLI) |

**Descripción.** Asociación MIME según D-20 sin reclamar text/plain genérico; verificación de Abrir con desde el gestor de archivos en nativo y Flatpak; documentación del flujo CLI soportado en Flatpak.

**Criterios de aceptación**
- [ ] Abrir un .env desde el gestor de archivos lanza Envy en ambos modos o la limitación queda documentada.
- [ ] Envy no aparece como editor por defecto de todos los archivos de texto.

**Pruebas:** Prueba manual en GNOME Files.

<a id="env-t082"></a>
#### ENV-T082 — Validar el flujo completo en Flatpak y la matriz de permisos bajo portal

| Campo | Valor |
|---|---|
| Épica | E9 Distribution & Release |
| Origen (requisito/decisión) | PRD §9, §14 (Flatpak validado); TD §12 (pruebas obligatorias), §14.2 |
| Prioridad | P0 |
| Estimación | 6 h |
| Depende de | ENV-T077, ENV-T061, ENV-T042, ENV-T060 |
| Bloquea a | ENV-T076, ENV-T083, ENV-T086 |
| Sprint propuesto | S13 (2027-02-15 → 2027-02-19) |
| Estado | Backlog |
| Agentes | QA, Security Reviewer |
| Entregables / archivos | Resultados en docs/testing/MANUAL_TESTS.md; lista de limitaciones |

**Descripción.** Agregar proyecto por selector > enumerar > editar > guardar > salir > reiniciar > reabrir proyecto > editar > modificación externa > detectar conflicto; además, conservación de 0600, solo lectura y enlace simbólico por portal. Registrar resultados y limitaciones.

**Criterios de aceptación**
- [ ] Cada paso de TD §12 tiene evidencia o la limitación queda registrada para decisión.
- [ ] Ningún caso termina con pérdida de datos.

**Pruebas:** Ejecución manual en sesión GNOME real.

<a id="env-t083"></a>
#### ENV-T083 — Redactar README, añadir la licencia y documentar limitaciones

| Campo | Valor |
|---|---|
| Épica | E9 Distribution & Release |
| Origen (requisito/decisión) | PRD §14 (README, captura, licencia); TD §15 Hito 4; decisión D-08 |
| Prioridad | P1 |
| Estimación | 5 h |
| Depende de | ENV-T082, ENV-T081 |
| Bloquea a | ENV-T084, ENV-T085 |
| Sprint propuesto | S14 (2027-02-22 → 2027-02-26) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | README.md, LICENSE, capturas |

**Descripción.** README con propósito, instalación nativa y Flatpak, compilación, CLI, privacidad, subconjunto de sintaxis y limitaciones conocidas; archivo LICENSE según D-08; capturas en claro y oscuro.

**Criterios de aceptación**
- [ ] Una persona sin contexto puede compilar y ejecutar siguiendo el README.
- [ ] Las limitaciones de T025/T054/T082 están documentadas.

**Pruebas:** Revisión siguiendo el README en un entorno limpio.

<a id="env-t084"></a>
#### ENV-T084 — Verificar el cumplimiento de requisitos de Flathub

| Campo | Valor |
|---|---|
| Épica | E9 Distribution & Release |
| Origen (requisito/decisión) | Distribución objetivo Flatpak/Flathub; decisión D-22 |
| Prioridad | P1 |
| Estimación | 4 h |
| Depende de | ENV-T077, ENV-T078, ENV-T079, ENV-T083 |
| Bloquea a | ENV-T085 |
| Sprint propuesto | S14 (2027-02-22 → 2027-02-26) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | Ajustes en manifiesto/metainfo |

**Descripción.** flatpak-builder-lint sobre manifiesto y repositorio, capturas y colores de marca en metainfo, justificación de permisos y requisitos de verificación del app ID.

**Criterios de aceptación**
- [ ] flatpak-builder-lint sin errores.
- [ ] Lista de requisitos de envío cumplidos o pendientes justificados.

**Pruebas:** Ejecución de los linters.

<a id="env-t085"></a>
#### ENV-T085 — Preparar el release candidate v0.1.0-rc

| Campo | Valor |
|---|---|
| Épica | E9 Distribution & Release |
| Origen (requisito/decisión) | PRD §11 hito 4 |
| Prioridad | P0 |
| Estimación | 3 h |
| Depende de | ENV-T084, ENV-T083 |
| Bloquea a | ENV-T076, ENV-T088 |
| Sprint propuesto | S14 (2027-02-22 → 2027-02-26) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | CHANGELOG.md, Cargo.toml, metainfo, checklist |

**Descripción.** Versión 0.1.0 en Cargo.toml y metainfo, CHANGELOG, borrador de notas, checklist de release y build RC (bundle Flatpak). La creación de etiquetas solo con aprobación del usuario.

**Criterios de aceptación**
- [ ] El build RC es reproducible desde un checkout limpio.
- [ ] El checklist de release está completo y enlazado.

**Pruebas:** Build limpio del RC en nativo y Flatpak.

<a id="env-t086"></a>
#### ENV-T086 — Corregir los defectos detectados en la validación Flatpak

| Campo | Valor |
|---|---|
| Épica | E9 Distribution & Release |
| Origen (requisito/decisión) | PRD §14; TD §16 |
| Prioridad | P0 |
| Estimación | 5 h |
| Depende de | ENV-T082 |
| Bloquea a | ENV-T076 |
| Sprint propuesto | S13 (2027-02-15 → 2027-02-19) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | Correcciones en src/ y packaging/ |

**Descripción.** Tiempo reservado para corregir defectos de T082 o ajustar permisos/documentación según las decisiones tomadas.

**Criterios de aceptación**
- [ ] Defectos P0 de Flatpak = 0 o limitación aprobada por el usuario.

**Pruebas:** Repetición de los casos fallidos de T082.

<a id="env-t087"></a>
#### ENV-T087 — Publicar v0.1.0 tras la aprobación go

| Campo | Valor |
|---|---|
| Épica | E9 Distribution & Release |
| Origen (requisito/decisión) | PRD §11; decisión D-22 |
| Prioridad | P0 |
| Estimación | 2 h |
| Depende de | ENV-T076, ENV-T088 |
| Bloquea a | Ninguna |
| Sprint propuesto | S15 (2027-03-01 → 2027-03-05) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | Release publicada |

**Descripción.** Con aprobación expresa del usuario: etiqueta, notas de release y artefactos; envío a Flathub según D-22 (la aceptación de Flathub no forma parte de la fecha).

**Criterios de aceptación**
- [ ] Los artefactos publicados corresponden al commit evaluado en T076.
- [ ] Existe aprobación explícita del usuario registrada.

**Pruebas:** Instalación del artefacto publicado en un entorno limpio.

<a id="env-t088"></a>
#### ENV-T088 — Corregir defectos bloqueantes del RC

| Campo | Valor |
|---|---|
| Épica | E9 Distribution & Release |
| Origen (requisito/decisión) | PRD §11 criterio de salida |
| Prioridad | P0 |
| Estimación | 4 h |
| Depende de | ENV-T085 |
| Bloquea a | ENV-T087 |
| Sprint propuesto | S15 (2027-03-01 → 2027-03-05) |
| Estado | Backlog |
| Agentes | Implementador, QA |
| Entregables / archivos | Correcciones |

**Descripción.** Tiempo reservado para corregir defectos bloqueantes detectados en T076 antes de publicar; si no se necesita, se libera capacidad.

**Criterios de aceptación**
- [ ] Defectos bloqueantes = 0 o release aplazado por decisión del usuario.

**Pruebas:** Repetición de casos fallidos de T076.

## 6. Fuera del alcance de v0.1.0 (no planificado)

Según PRD §2.2 y TD §16: cuentas/nube/sincronización; SSH, Docker, Kubernetes, CI/CD o proveedores de secretos; ejecución de comandos o inyección de variables; expansión/interpretación de shell; drag & drop y reordenamiento de filas; iconos por servicio; histórico persistente; comparación valor a valor entre entornos; escaneo recursivo; respaldos automáticos; merge de tres vías; recursos remotos/GVfs. Las ideas futuras se registran como propuestas en [DECISIONS.md §4](DECISIONS.md#4-propuestas-para-versiones-futuras-no-v010), nunca como tareas de v0.1.0.
