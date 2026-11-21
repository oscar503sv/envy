# Envy v0.1.0 — Definition of Ready, Definition of Done y criterios de release

> **Estado:** propuesta pendiente de aprobación.  
> **Base:** PRD §10 (métricas de calidad), §11 (criterio de salida), §14 (DoD del producto); TD §14 (estrategia de pruebas).  
> **Relacionados:** [BACKLOG](BACKLOG.md) · [MILESTONES](MILESTONES.md) · [TASK_TEMPLATE](TASK_TEMPLATE.md)

## 1. Definition of Ready (tarea preparada)

Una tarea pasa de `Backlog` a `Ready` solo si:

- [ ] Tiene ID, título accionable, origen (requisito/decisión) y prioridad.
- [ ] Sus dependencias están en `Done` (o se ejecutan antes dentro del mismo sprint).
- [ ] Las decisiones de DECISIONS.md que la afectan están aprobadas **o** el usuario ha aceptado proceder con la recomendación por defecto (se anota en la tarea).
- [ ] Los criterios de aceptación son observables y las pruebas a ejecutar están definidas.
- [ ] La estimación es ≤ 8 h; si no, se divide.
- [ ] Están identificados los archivos/componentes afectados (o la tarea de exploración que los identificará).
- [ ] Se sabe si requiere Security Reviewer.
- [ ] El paquete de contexto de [TASK_TEMPLATE.md](TASK_TEMPLATE.md) está preparado.

## 2. Definition of Done — tarea

Una tarea pasa a `Done` solo con **evidencia verificable** (no basta con que exista un archivo o con una afirmación):

- [ ] Todos sus criterios de aceptación cumplidos, con evidencia enlazada (salida de comandos, test, captura o informe).
- [ ] `cargo fmt --check`, `cargo clippy -- -D warnings` y `cargo test` en verde localmente y en CI (cuando exista CI).
- [ ] Pruebas nuevas cubren el comportamiento añadido, incluidos caminos de error y casos límite; cada defecto corregido añade una prueba que lo reproduce.
- [ ] Fidelidad: `tests/fidelity.rs` sigue al 100 % (desde ENV-T012).
- [ ] Ningún valor de variable ni contenido de archivo en logs, errores, Debug, panics, configuración o salida de tests (centinela ausente).
- [ ] Sin `unwrap()`/`expect()` en rutas de E/S o datos de usuario sin justificación documentada.
- [ ] Cadenas visibles marcadas para traducción (desde ENV-T033).
- [ ] Revisión humana del diff completo; Security Reviewer aprobado si la tarea lo exige.
- [ ] Sin cambios fuera del alcance de la tarea; ninguna dependencia nueva sin aprobación.
- [ ] Documentación afectada actualizada (SYNTAX, README, guion manual) cuando aplica.
- [ ] Estado actualizado en el backlog con horas reales.

Para **spikes**: hipótesis respondida, evidencia adjunta, propuesta registrada en DECISIONS.md y código desechable fuera de `main`.

## 3. Definition of Done — sprint

- [ ] Todas las tareas del sprint en `Done` o devueltas al backlog con motivo y nueva estimación.
- [ ] El incremento es demostrable (demo del SPRINT_PLAN ejecutada) y `main` compila y pasa la suite.
- [ ] Defectos P0 abiertos introducidos en el sprint = 0.
- [ ] Horas reales registradas; desvío respecto a lo planificado calculado.
- [ ] Riesgos y decisiones revisados; nuevas decisiones registradas.
- [ ] Siguiente sprint replanificado si hubo desvío.

## 4. Definition of Done — hito

Cada hito de [MILESTONES.md](MILESTONES.md) se da por cumplido solo cuando todos sus criterios de salida tienen evidencia y el usuario lo acepta. La fecha estimada no se convierte en compromiso sin aprobación.

## 5. Definition of Done — MVP v0.1.0 (mapa de PRD §14)

| Criterio PRD §14 | Evidencia exigida | Tareas |
|---|---|---|
| Fidelidad byte a byte de todos los fixtures sin cambios | `tests/fidelity.rs` al 100 % + proptest sin fallos | ENV-T012, ENV-T013 |
| Edición simple y multilínea cubierta con tests | `tests/editing.rs` | ENV-T016, ENV-T068 |
| Crear/eliminar/pegar/copiar/undo/redo con tests | `tests/editing.rs`, test de portapapeles | ENV-T017–T021, ENV-T049, ENV-T068 |
| El guardado nunca sobrescribe silenciosamente cambios externos | Matriz de guardado y conflictos en verde | ENV-T067, ENV-T069 |
| Quick Edit fuera de Workspace en instalación nativa | Situación A en guion manual | ENV-T053 |
| Proyectos, servicios, archivos, comparación y duplicados funcionan | Situaciones B, C, E | ENV-T063 |
| Flatpak validado en entorno real o limitaciones comunicadas y resueltas | Resultados TD §12 | ENV-T082, ENV-T086 |
| UI GNOME adaptable, con teclado y sin contenido sensible en logs | Auditoría A11y + auditoría de fugas con UI | ENV-T062, ENV-T073, ENV-T075 |
| README, captura, licencia y metadatos | Archivos validados | ENV-T078, ENV-T079, ENV-T083 |

## 6. Puertas de calidad

| Puerta | Cuándo | Condición |
|---|---|---|
| G-PR | Cada cambio | fmt + clippy + test en verde; revisión humana; Security Reviewer si aplica |
| G-FID | Cada cambio desde ENV-T012 | Fidelidad 100 % |
| G-SEC | Cada cambio en E/S, logs, portapapeles, configuración, pegado | Security Reviewer aprobado; centinela ausente |
| G-UI | Cada cambio de UI desde ENV-T005 | Pruebas GTK ejecutadas con display (no omitidas en silencio) |
| G-HITO | Fin de hito | Criterios de salida del hito con evidencia |
| G-RC | ENV-T076 | Criterios de release §7 |

## 7. Criterios de release / no-release

**Release (go)** — todos deben cumplirse:

- [ ] Todos los requisitos P0, P1 y P2 del PRD §5 completados o renegociados explícitamente en el PRD (PRD §11).
- [ ] 100 % de fixtures idénticos byte a byte.
- [ ] Ninguna prueba ni escenario manual permite perder ediciones locales o externas sin acción expresa.
- [ ] Auditoría de fugas (núcleo + UI) sin hallazgos.
- [ ] Todas las acciones esenciales completables con teclado; estados y etiquetas con texto.
- [ ] Flujo Flatpak de TD §12 validado o limitaciones aprobadas por el usuario y documentadas.
- [ ] Decisiones bloqueantes cerradas: D-07, D-08, D-12, D-19, D-22.
- [ ] Metadatos validados (desktop, AppStream) y `flatpak-builder-lint` sin errores.
- [ ] README con instrucciones reproducibles y limitaciones.
- [ ] Aprobación explícita del usuario (go).

**No-release (no-go)** — basta uno:

- Cualquier regresión de fidelidad de archivo, de conflictos o de secretos (PRD §11).
- Un caso conocido de sobrescritura silenciosa, truncado o pérdida de datos.
- Un valor secreto aparece en logs, errores, configuración o salida de tests.
- Envy informa "Guardado" sin confirmación de escritura.
- Un enlace simbólico se reemplaza por un archivo regular.
- Permisos solicitados en Flatpak sin justificar o ampliados en silencio.
- Pruebas GTK reportadas como aprobadas sin haberse ejecutado.

## 8. Lista de comprobación específica de archivos (PRD §8, TD §8, §14)

| Aspecto | Verificación automatizada | Verificación manual | Tareas |
|---|---|---|---|
| Comentarios, espacios y saltos de línea | Fidelidad + proptest | — | ENV-T012, ENV-T013 |
| Comillas y multilínea | Fixtures + tests de codec y SetValue | Edición multilínea en UI | ENV-T011, ENV-T014, ENV-T038, ENV-T068 |
| Sintaxis desconocida | Fixture unknown-syntax, nodos Raw | Filas Raw no editables | ENV-T010, ENV-T036 |
| UTF-8 y entradas no válidas | invalid-utf8.bin, proptest | Página de estado | ENV-T009, ENV-T013, ENV-T041 |
| Variables duplicadas | duplicate-keys, eliminación selectiva, pegado | Avisos ×n | ENV-T018, ENV-T021, ENV-T066 |
| Undo/redo | Secuencias de historial | Foco y Ctrl+Z | ENV-T019, ENV-T039, ENV-T046 |
| Cambios externos durante la edición | tests/file_conflicts.rs | Edición concurrente | ENV-T069, ENV-T042 |
| Conflictos con cambios locales | Máquina de estados | Banner y resolución | ENV-T029, ENV-T042 |
| Sobrescrituras y pérdida de datos | Matriz de guardado | Cierre con cambios | ENV-T067, ENV-T044 |
| Conservación de permisos | 0600/0644 en matriz | Portal Flatpak | ENV-T067, ENV-T082 |
| Enlaces simbólicos | Rechazo en lectura y guardado | Selección explícita del destino | ENV-T027, ENV-T043, ENV-T067 |
| Errores de E/S | Sin permiso, directorio sin creación, archivo eliminado | Mensajes legibles | ENV-T027, ENV-T067 |
| Secretos en logs | tests/secrets_leak.rs | Revisión de tracing | ENV-T070, ENV-T075 |
| Flatpak/portales | — | Flujo TD §12 | ENV-T054, ENV-T082 |
| Accesibilidad y GNOME nativo | Pruebas GTK básicas | Orca, teclado, contraste, ventana estrecha | ENV-T062, ENV-T072, ENV-T073 |
| Regresión | Suite completa en CI | Guion manual | ENV-T071, ENV-T075, ENV-T076 |
