# Envy v0.1.0 — Registro de decisiones

> **Estado (plan v2, 2026-11-20):** resueltas **D-01**, **D-02**, **D-03** y **D-32** (aprobadas por el usuario). Todas las demás decisiones siguen **pendientes** o **propuestas**; no se atribuyen aprobaciones inexistentes.  
> **Regla:** este documento no modifica [`docs/PRD.md`](../PRD.md) ni [`docs/TECHNICAL_DESIGN.md`](../TECHNICAL_DESIGN.md). Los cambios que esos documentos necesiten se registran como propuestas en la §3 para que el usuario los apruebe y los aplique.  
> **Relacionados:** [BACKLOG](BACKLOG.md) · [SPRINT_PLAN](SPRINT_PLAN.md) · [RISKS](RISKS.md) · [MILESTONES](MILESTONES.md)

## 1. Base arquitectónica tomada del diseño técnico

Los ADR-001…ADR-010 de TD §2 (crate único modular, núcleo sin GTK, edición por fragmentos/rangos, comandos tipados, `DocumentSession` por archivo, Gio para URI/monitor/guardado, configuración sin secretos, acceso explícito a carpetas, sin parser universal, backend con capacidades) se usan como **base de planificación**. El TD declara su estado como *diseño objetivo* y el PRD como *especificación propuesta*: el plan no los trata como aprobados formalmente, pero tampoco los cambia. Cualquier desviación se registra aquí antes de aplicarse.

### Decisiones de planificación propuestas (P-ADR)

| ID | Decisión propuesta | Justificación | Estado |
|---|---|---|---|
| P-ADR-01 | Scrumban ligero con WIP = 1 tarea de implementación y una tarea de QA/revisión en paralelo como máximo | Un único desarrollador: el paralelismo solo crea trabajo a medias | Propuesto |
| P-ADR-02 | Núcleo puro (motor + persistencia + conflictos) antes de la UI | Secuencia recomendada en TD §15 y PRD §11; los P0 se prueban sin GTK | Propuesto |
| P-ADR-03 | Spikes de riesgo adelantados: guardado Gio (ENV-T025, S02) y Flatpak (ENV-T054, S03) | Sus resultados pueden cambiar el diseño de guardado y de Workspace; descubrirlo tarde es caro | Propuesto |
| P-ADR-04 | Lógica pura de duplicados y comparación de claves en la fase de motor; su UI en la fase Workspace | La reutilizan pegado (P1) y revisión de conflicto (P0); ver D-26 | Propuesto |
| P-ADR-05 | El backlog se genera y verifica desde una única fuente de datos para que BACKLOG.md y SPRINT_PLAN.md no diverjan | Consistencia de totales | Propuesto |
| P-ADR-06 | Puertas de aprobación: toda tarea que dependa de una propuesta pendiente de aprobación (ENV-T001, ENV-T006, ENV-T025, ENV-T039, ENV-T054) se planifica en un sprint posterior al de la tarea que la produce | Con 41 h/semana un sprint contiene muchas tareas; sin esta regla el código se escribiría antes de que el usuario decida | Propuesto (plan v2) |

## 2. Decisiones pendientes

Cada decisión indica: contexto, opciones, recomendación, impacto si se retrasa y fecha límite derivada del plan. **Fecha límite** = inicio del sprint de la primera tarea afectada. Los IDs son estables; D-30 y D-31 no se usan (descartados durante el análisis por estar cubiertos por D-20 y por ENV-T015).

### 2.1 Conflictos y ambigüedades entre documentos

<a id="d-01"></a>
#### D-01 — Prefijo de los IDs de tarea (conflicto con IDs de requisitos) — **RESUELTA**
- **Resolución (2026-11-20):** aprobada la opción (a), `ENV-T###` para tareas. En Orbit se mantiene la equivalencia ENVY-n = ENV-T(n−9).
- **Contexto:** el encargo pide IDs `ENV-001`, pero el PRD §5 ya usa `ENV-001…ENV-020` para **requisitos**. Reutilizar el formato haría que `ENV-010` significara dos cosas.
- **Opciones:** (a) `ENV-T###` para tareas; (b) `ENVY-###`; (c) renumerar requisitos del PRD (no recomendado: modifica el PRD).
- **Recomendación:** (a), ya aplicada de forma provisional en el backlog. Cambiarla es un renombrado mecánico.
- **Fecha límite:** antes de crear tareas en cualquier herramienta de seguimiento (S01).

<a id="d-05"></a>
#### D-05 — Versión mínima de libadwaita (incoherencia interna del TD)
- **Contexto:** TD §3 y §10.2 fijan **1.4** como mínimo, pero el mismo TD sugiere `AdwAlertDialog` y `AdwPreferencesDialog`, que se introdujeron en **libadwaita 1.5**. El TD ya prevé "subirla explícitamente si algún otro widget lo requiere".
- **Opciones:** (a) mínimo 1.5 o el que traiga el runtime GNOME elegido (D-06); (b) mantener 1.4 y usar `AdwMessageDialog` (obsoleto desde 1.6).
- **Recomendación:** (a); se confirma con ENV-T001. Para Flatpak manda el runtime; para instalación nativa, la distribución de prueba oficial.
- **Fecha límite:** S07 (ENV-T033), 2027-01-04. Propuesta de cambio al TD en §3.

<a id="d-10"></a>
#### D-10 — Almacenamiento de configuración: TOML y/o GSettings
- **Contexto:** TD §3 dice "`serde` + `toml` **o** GSettings" y TD §4 incluye un `gschema.xml`; TD §11.2 recomienda TOML para el catálogo y GSettings para preferencias simples.
- **Opciones:** (a) TOML para catálogo y última vista, GSettings solo para estado de ventana; (b) solo TOML (sin gschema); (c) solo GSettings.
- **Recomendación:** (b) si no hay preferencias reales en v0.1 (ver D-28); (a) si se quiere recordar el tamaño de ventana. Menos piezas = menos riesgo en Flatpak.
- **Fecha límite:** S10 (ENV-T055), 2027-01-25; afecta a ENV-T078 (S13).

<a id="d-14"></a>
#### D-14 — ¿Se ofrece "Sobrescribir disco" en un conflicto?
- **Contexto:** PRD US-05 dice "la opción de sobrescribir el disco, **si se ofrece**, requiere confirmación… y una nueva comprobación"; TD §7.2 no la incluye.
- **Opciones:** (a) no ofrecerla en v0.1 (solo Recargar y descartar, Guardar como, Revisar); (b) ofrecerla con doble confirmación y nueva comprobación (+4–6 h estimadas, más pruebas).
- **Recomendación:** (a). Reduce superficie de pérdida de datos; Guardar como cubre el caso.
- **Fecha límite:** S08 (ENV-T042), 2027-01-11.

<a id="d-15"></a>
#### D-15 — Modelo de múltiples documentos abiertos
- **Contexto:** PRD US-01: si hay otro archivo con cambios, "abre el nuevo documento en una sesión/vista independiente **o** pide una decisión apropiada". TD §9: una instancia primaria; no se reemplazan sesiones.
- **Opciones:** (a) una ventana por documento en Quick Edit; la ventana Workspace mantiene varias sesiones en memoria; (b) pestañas (`AdwTabView`) en una ventana; (c) preguntar al usuario.
- **Recomendación:** (a): la más simple con `GApplication::open` y sin pérdida. (b) añade alcance de UI no pedido.
- **Fecha límite:** S07 (ENV-T035), 2027-01-04.

<a id="d-16"></a>
#### D-16 — Diálogo al cerrar con cambios sin guardar (requisito derivado)
- **Contexto:** el PRD no lo lista en §5, pero PRD §10 exige "ningún test permite descartar ediciones locales… sin una acción expresa". Cerrar la ventana sin aviso violaría esa métrica.
- **Recomendación:** incluirlo (ENV-T044, 4 h) como requisito derivado de PRD §10 y añadirlo explícitamente al PRD (§3).
- **Fecha límite:** S09 (ENV-T044), 2027-01-18.

<a id="d-17"></a>
#### D-17 — Alcance del revelado/ocultado manual por fila
- **Contexto:** PRD ENV-012 y §8.3 hablan de heurística "configurable por fila". No se especifica si la elección persiste. ADR-007 prohíbe valores en configuración; persistir nombres de clave por archivo no viola ADR-007, pero amplía el catálogo.
- **Opciones:** (a) solo durante la sesión (en memoria); (b) persistir en el catálogo por URI + clave.
- **Recomendación:** (a) para v0.1; (b) como propuesta futura.
- **Fecha límite:** S09 (ENV-T048), 2027-01-18.

<a id="d-18"></a>
#### D-18 — Comparación con `.env.example` en Quick Edit
- **Contexto:** PRD ENV-018 la describe sin limitarla a Workspace; en Flatpak, abrir un solo archivo por portal **no** da acceso a sus hermanos (PRD §9, TD §12).
- **Recomendación:** disponible cuando el `.env.example` de la misma carpeta sea legible (nativo y proyectos de Workspace); en Quick Edit Flatpak sin acceso a la carpeta, mostrar por qué no está disponible.
- **Fecha límite:** S12 (ENV-T065), 2027-02-08.

<a id="d-26"></a>
#### D-26 — Orden de construcción de duplicados y comparación (PRD §11 vs TD §15)
- **Contexto:** PRD §11 sitúa duplicados y comparación en el hito Workspace; TD §15 Hito 1 incluye "deduplicación diagnóstica y comparación de claves" en el núcleo.
- **Resolución propuesta:** no hay contradicción de alcance: la lógica pura (ENV-T020, ENV-T023) se construye en la fase de motor porque la necesitan el pegado y la revisión de conflictos; la UI (ENV-T065, ENV-T066) se construye en la fase Workspace. Requiere solo conformidad del usuario.

<a id="d-27"></a>
#### D-27 — Semántica de "Guardar como…"
- **Contexto:** TD §7.2 dice "guardar copia independiente sin alterar destino conflictivo"; no indica si la sesión pasa a editar el archivo nuevo.
- **Opciones:** (a) la sesión pasa a apuntar al archivo nuevo (comportamiento habitual en GNOME); (b) la sesión sigue en el original y la copia es un volcado.
- **Recomendación:** (a), con el original intacto y su conflicto cerrado para esa sesión.
- **Fecha límite:** S05 (ENV-T030), 2026-12-21 (semana festiva).

<a id="d-28"></a>
#### D-28 — Diálogo de preferencias
- **Contexto:** TD §10.2 lista `AdwPreferencesDialog` ("configuración mínima"); el PRD no define ninguna preferencia en §5.
- **Recomendación:** no incluir diálogo de preferencias en v0.1 (no está en el alcance del PRD). Si el usuario lo quiere, se añade vía PRD.
- **Fecha límite:** S07, 2027-01-04.

<a id="d-29"></a>
#### D-29 — Patrón `.env.*` frente a la exclusión de `.env.example.txt` (contradicción interna del PRD)
- **Contexto:** PRD US-06 admite "exactamente `.env`, `.env.*`, `*.env`" y a la vez dice que no se confunde `.env.example.txt`, que **sí** coincide literalmente con `.env.*`. (`.envrc` no coincide, sin conflicto.)
- **Opciones:** (a) `.env.*` excepto nombres cuya última extensión esté en una lista corta documentada (p. ej. `.txt`, `.md`, `.bak`, `.orig`, `.swp`); (b) `.env.*` sin excepciones y retirar el ejemplo del PRD; (c) limitar `.env.*` a sufijos sin punto adicional (rompe `.env.production.local`).
- **Recomendación:** (a), con la lista registrada en el PRD.
- **Fecha límite:** S10 (ENV-T057), 2027-01-25.

### 2.2 Decisiones pendientes declaradas en PRD §13 / TD §16

<a id="d-06"></a>
#### D-06 — Runtime GNOME/SDK y distribución de pruebas oficial
- **Contexto:** PRD §13, §9. Determina libadwaita disponible (D-05) y el entorno de regresión (ENV-T075).
- **Recomendación:** runtime GNOME estable vigente en Flathub en el momento de ENV-T001 y una distribución de referencia (p. ej. Fedora Workstation actual) para pruebas nativas. Se propone con evidencia en ENV-T001.
- **Fecha límite:** S07 (ENV-T033), 2027-01-04; obligatoria en S12 (ENV-T077), 2027-02-08.

<a id="d-07"></a>
#### D-07 — App ID, propietario e identidad del proyecto
- **Contexto:** PRD §13: `io.github.<owner>.Envy` es provisional; hay que verificar el nombre en tiendas/repositorios.
- **Impacto:** el app ID aparece en GApplication, GResource, desktop, metainfo, gschema, icono y manifiesto. Cambiarlo tarde implica renombrados en muchos archivos y en la ruta de configuración del sandbox.
- **Fecha límite recomendada:** S07 (ENV-T033), 2027-01-04; **obligatoria:** S12 (ENV-T077), 2027-02-08 (ENV-T078 en S13).

<a id="d-08"></a>
#### D-08 — Confirmar licencia GPL-3.0-or-later
- **Fecha límite:** antes de publicar cualquier repositorio público; como muy tarde S14 (ENV-T083), 2027-02-22.

<a id="d-11"></a>
#### D-11 — Subconjunto de sintaxis dotenv v0.1
- **Contexto:** PRD §13 deja abiertos `export`, escapes, comentarios finales y otros dialectos. ENV-T006 redacta la propuesta (docs/SYNTAX.md) con preguntas concretas, entre ellas:
  - Escapes admitidos en comillas dobles y si las simples son literales.
  - Cuándo un `#` sin comillas inicia comentario (¿exige espacio previo?).
  - Tratamiento de comillas sin cerrar (¿Raw solo de esa línea o hasta EOF?).
  - Representación del BOM en el modelo de nodos.
  - Salto de línea de contenido insertado o pegado (estilo dominante) y regla de newline final al añadir/eliminar la última línea.
- **Fecha límite:** S02 (ENV-T010), **2026-11-30**. ENV-T006 se entrega en S01 y ENV-T010 es la tercera tarea de S02, lo que deja 1–2 días de margen. Si no se aprueba a tiempo, se implementa la propuesta y se asume el riesgo de retrabajo (R-08).

<a id="d-12"></a>
#### D-12 — Política de guardado cuando faltan garantías
- **Contexto:** PRD §13 y TD §8.2/§16. Puntos a decidir tras ENV-T025:
  1. **Ventana de carrera residual:** en sistemas POSIX locales no hay reemplazo condicional atómico general; comprobación + reemplazo deja una ventana TOCTOU mínima (TD §8.2 ya lo reconoce). Además, en backends locales el etag de Gio suele derivarse del mtime (a verificar). Propuesta: hash + etag inmediatamente antes, reemplazo con etag, verificación posterior y **documentar la limitación** sin prometer atomicidad concurrente.
  2. **Permisos no conservados** (p. ej. 0600 → 0644): propuesta: tratarlo como error visible y no declarar éxito; restaurar el modo si es posible y verificable.
  3. **Sin reemplazo atómico** (p. ej. caída a escritura en el sitio): propuesta: bloquear el guardado normal y ofrecer Guardar como.
- **Fecha límite:** S05 (ENV-T028), **2026-12-21** (semana festiva). El spike ENV-T025 termina en S02 (2026-12-04).

<a id="d-19"></a>
#### D-19 — Política de acceso a carpetas en Flatpak
- **Contexto:** PRD §13, §8.9; TD §12. Depende del spike ENV-T054.
- **Opciones:** (a) solo portales (selección de carpeta + persistencia del document portal); (b) portales + permiso ampliado documentado y justificado (p. ej. `--filesystem=home`), nunca silencioso; (c) Workspace limitado en Flatpak con limitaciones comunicadas.
- **Recomendación:** (a) si el spike lo valida; si no, presentar (b)/(c) al usuario con trade-offs. Bloquea el release Flatpak con Workspace (TD §16).
- **Fecha límite:** S10 (ENV-T055), **2027-01-25**. El spike ENV-T054 termina en S03 (2026-12-11).

<a id="d-20"></a>
#### D-20 — Asociación de archivos (MIME) y CLI en Flatpak
- **Contexto:** PRD §7.1, TD §9: no reclamar `text/plain` genérico. Puede requerir definir un tipo MIME propio por patrón de nombre o limitarse a "Abrir con".
- **Recomendación:** decidir con la evidencia de ENV-T054 y validar en ENV-T081; documentar el flujo CLI Flatpak (`flatpak run` con reenvío de archivos) como experiencia separada (PRD US-01).
- **Fecha límite:** S13 (ENV-T081), 2027-02-15.

### 2.3 Decisiones de planificación y de implementación

<a id="d-02"></a>
#### D-02 — Capacidad semanal real — **RESUELTA**
- **Resolución:** **41 h/semana brutas**, confirmadas por el usuario (comunicado por el coordinador el 2026-11-20). Con la reserva del 20 % (D-35) quedan 32,8 h netas por semana (26,24 h en las semanas festivas).
- **Efecto:** plan v2 de 15 sprints con v0.1.0 estimada el 2027-03-05 (plan v1 con 15 h supuestas: 38 sprints, 2027-08-13). Las estimaciones por tarea no cambian (88 tareas / 417 h).
- **Seguimiento:** la sostenibilidad de este ritmo se vigila como riesgo R-17; si la capacidad real observada difiere, se replanifica con el valor real.

<a id="d-03"></a>
#### D-03 — Festivos y ausencias — **RESUELTA**
- **Resolución (2026-11-20):** el usuario confirma el supuesto: solo se descuentan el 2026-12-25 y el 2027-01-01. El calendario de SPRINT_PLAN no cambia.
- **Supuesto:** solo se descuentan el 2026-12-25 y el 2027-01-01 (1 día cada uno, es decir 8,2 h brutas por semana festiva). El plan v2 termina el 2027-03-05, antes de Semana Santa 2027; no se han considerado festivos regionales ni ausencias. Con 41 h/semana es probable que el usuario quiera más días libres en Navidad: confirmar el calendario personal (afecta a S05–S06 y a DG-4/DG-5).

<a id="d-04"></a>
#### D-04 — Semántica de prioridades en el backlog
- **Propuesta:** usar la semántica del PRD §5 (P0 seguridad/corrección, P1 experiencia esencial, P2 Workspace; todas en el MVP). Las tareas habilitadoras heredan la prioridad más alta de lo que desbloquean.

<a id="d-09"></a>
#### D-09 — Sistema de build: Cargo + build.rs o Meson
- **Contexto:** TD §4 muestra una estructura solo con Cargo; las apps GNOME suelen usar Meson para instalar desktop/metainfo/gschema/iconos y traducciones.
- **Opciones:** (a) Cargo + build.rs (glib-build-tools) + instalación desde el manifiesto Flatpak; (b) Meson envolviendo Cargo.
- **Recomendación:** se propone en ENV-T001 con evidencia. (a) es más simple al principio; (b) simplifica i18n e instalación nativa.
- **Fecha límite:** S07 (ENV-T033), 2027-01-04. Propuesta producida por ENV-T001 en S01.

<a id="d-13"></a>
#### D-13 — Política única de undo (Ctrl+Z en campos)
- **Contexto:** TD §6.2 exige evitar dos historiales contradictorios (el del widget GTK y el de Envy). Lo resuelve el spike ENV-T039.
- **Fecha límite:** S08 (ENV-T037), 2027-01-11. El spike ENV-T039 se ejecuta en S07.

<a id="d-21"></a>
#### D-21 — Alcance de traducciones e idioma fuente de la UI
- **Contexto:** PRD §11 hito 4 pide "traducciones preparadas"; no dice qué idiomas se entregan.
- **Recomendación:** cadenas fuente en inglés (convención GNOME), infraestructura gettext preparada; incluir traducción al español solo si hay holgura y se aprueba.
- **Fecha límite:** S13 (ENV-T080), 2027-02-15.

<a id="d-22"></a>
#### D-22 — Qué significa "v0.1.0 publicada"
- **Contexto:** la distribución objetivo es Flathub, pero la revisión de Flathub no depende del proyecto.
- **Recomendación:** v0.1.0 = etiqueta + notas + artefactos (código fuente y bundle Flatpak) con el manifiesto validado por `flatpak-builder-lint`; el envío a Flathub se hace a continuación y su aceptación **no** forma parte de la fecha estimada.
- **Fecha límite:** S14, 2027-02-22.

<a id="d-23"></a>
#### D-23 — Plataforma de CI
- **Supuesto:** GitHub Actions (coherente con el app ID provisional `io.github.*`). Confirmar.
- **Fecha límite:** S02 (ENV-T003), 2026-11-30.

<a id="d-24"></a>
#### D-24 — Adoptar `proptest` como dev-dependency
- **Contexto:** TD §3 la marca como opcional. El parser es la pieza P0 más crítica.
- **Recomendación:** adoptarla (solo en tests).
- **Fecha límite:** S02 (ENV-T013), 2026-11-30.

<a id="d-25"></a>
#### D-25 — Implementación del hash de versión
- **Opciones:** (a) crate `sha2` en el núcleo (mantiene ADR-002 sin GLib en domain); (b) `glib::Checksum` en infraestructura (sin dependencia nueva, pero el hash queda fuera del núcleo puro).
- **Recomendación:** decidir en ENV-T026 revisando qué hay ya en el árbol de dependencias.
- **Fecha límite:** S04 (ENV-T026), 2026-12-14.

<a id="d-32"></a>
#### D-32 — Edición de Rust y MSRV — **RESUELTA**
- **Resolución (2026-11-20):** aprobada la edición 2024 sobre Rust estable. ENV-T001 fija el MSRV concreto y verifica que la extensión rust-stable del SDK Flatpak la soporta; si no la soporta, se vuelve a consultar.
- **Contexto:** TD §3: edición 2024 "si el toolchain objetivo la soporta". Se confirma en ENV-T001 junto con la extensión rust-stable del SDK Flatpak.
- **Fecha límite:** S01.

<a id="d-35"></a>
#### D-35 — Mantener la reserva del 20 % con 41 h/semana
- **Contexto:** al pasar a 41 h/semana se revisa si el 20 % de reserva (8,2 h/semana) sigue siendo adecuado.
- **Propuesta:** mantenerla. La revisión humana de la salida de los agentes, la planificación semanal y la coordinación crecen con el volumen de trabajo; además, a este ritmo la fatiga reduce la calidad de la revisión (R-17). No se reduce para "ganar" calendario.
- **Estado:** propuesto (aplicado en el plan v2).

<a id="d-33"></a>
#### D-33 — Presentación de archivos con UTF-8 no válido
- **Contexto:** PRD US-02: se rechaza la edición estructurada indicando la razón.
- **Recomendación:** página de estado explicativa sin mostrar contenido (evita una visualización con pérdida); el archivo no se toca.
- **Fecha límite:** S07 (ENV-T041), 2027-01-04.

<a id="d-34"></a>
#### D-34 — Enlaces duros (condicional al spike ENV-T025)
- **Contexto:** el PRD no menciona enlaces duros. Si el spike confirma que el reemplazo por renombrado los rompe, hay que decidir entre documentarlo como limitación o tratarlos como los enlaces simbólicos (aviso). No se planifica trabajo hasta conocer el resultado.
- **Fecha límite:** S05, 2026-12-21.

## 3. Propuestas de cambio a PRD y TD (no aplicadas)

| # | Documento | Sección | Cambio propuesto | Decisión |
|---|---|---|---|---|
| C-01 | PRD | §6 US-06 | Precisar la regla de `.env.*` y la lista de exclusiones (o retirar el ejemplo `.env.example.txt`) | D-29 |
| C-02 | PRD | §5 / §6 US-01 | Añadir protección explícita al cerrar con cambios sin guardar | D-16 |
| C-03 | PRD | §6 US-05 | Fijar si existe la opción "Sobrescribir disco" en v0.1 | D-14 |
| C-04 | PRD | §13 | Registrar el subconjunto de sintaxis aprobado y enlazar docs/SYNTAX.md | D-11 |
| C-05 | PRD | §13 | Registrar la política de guardado sin garantías y la limitación TOCTOU | D-12 |
| C-06 | PRD | §6 US-07 | Precisar disponibilidad de la comparación con `.env.example` en Quick Edit/Flatpak | D-18 |
| C-07 | TD | §3, §10.2 | Subir la versión mínima de libadwaita a la que incluya `AdwAlertDialog` (≥ 1.5) o al runtime elegido | D-05 |
| C-08 | TD | §3, §11.2 | Resolver TOML vs GSettings | D-10 |
| C-09 | TD | §4 | Añadir `docs/SYNTAX.md`, `docs/testing/`, `examples/`, `po/` y el sistema de build elegido | D-09, D-11 |
| C-10 | TD | §8.2 | Documentar el resultado del spike de guardado y la garantía residual real | D-12 |
| C-11 | TD | §10.2 | Retirar o aplazar `AdwPreferencesDialog` si no hay preferencias en v0.1 | D-28 |
| C-12 | TD | §7.2 | Precisar la semántica de la sesión tras "Guardar como…" | D-27 |

<a id="4-propuestas-para-versiones-futuras-no-v010"></a>
## 4. Propuestas para versiones futuras (no v0.1.0)

Solo ideas que el propio PRD/TD sitúan fuera del MVP o que surgieron al planificar; **no** se planifican ni estiman para v0.1.0:

- Respaldos opt-in y explícitos (PRD §8.4).
- Reordenamiento de filas / drag & drop (PRD §2.2, TD §16).
- Iconos por servicio (PRD §2.2, TD §16).
- Comparación valor a valor entre entornos e histórico persistente (PRD §2.2).
- Merge asistido de conflictos (TD §7.3: v0.1 solo revisa).
- Recursos remotos/GVfs (PRD §9).
- Persistencia de la preferencia de ocultación por clave (D-17 opción b).
- Diálogo de atajos de teclado (no requerido por PRD §7.4).
