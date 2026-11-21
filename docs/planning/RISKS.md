# Envy v0.1.0 — Registro de riesgos

> **Estado:** actualizado para el plan v2 (2026-11-20, 41 h/semana). Revisión semanal en la planificación del sprint.  
> **Escalas:** Probabilidad (P) y Impacto (I): Baja / Media / Alta. **Exposición** = combinación cualitativa.  
> **Relacionados:** [BACKLOG](BACKLOG.md) · [SPRINT_PLAN](SPRINT_PLAN.md) · [DECISIONS](DECISIONS.md) · [MILESTONES](MILESTONES.md)

## 1. Top 5 riesgos de calendario

Actualizado para el plan v2 (41 h/semana confirmadas, 15 sprints).

| # | Riesgo | P | I | Mitigación principal |
|---|---|---|---|---|
| 1 | **R-17** Sostenibilidad de 41 h/semana durante 15 semanas seguidas con un único desarrollador (fatiga, caída de la calidad de revisión) | Alta | Alta | Reserva del 20 % intacta, WIP = 1, horas reales registradas semanalmente, replanificar con la capacidad real observada |
| 2 | **R-08** Latencia de decisiones del usuario: con sprints densos, cada decisión llega antes (D-11 el 2026-11-30; D-12 y DG-5 en semanas festivas) | Alta | Media | Puertas de aprobación (P-ADR-06), fechas límite en MILESTONES §2, recomendación por defecto si no hay respuesta |
| 3 | **R-04** Complejidad del parser/ValueCodec (escapes, multilínea, dialectos) en la ruta crítica | Media | Alta | Especificación primero (ENV-T006), fixtures, proptest, fallback a `Raw`; reestimar si se supera un 50 % |
| 4 | **R-02** Portales Flatpak no permiten Workspace persistente sin permisos ampliados | Media | Alta | Spike ENV-T054 en S03; decisión D-19 antes del 2027-01-25 |
| 5 | **R-01** No existe garantía de guardado condicionado suficiente con Gio en local | Alta | Alta | Spike ENV-T025 en S02; política D-12 antes del 2026-12-21; bloqueo + Guardar como |

R-09 (errores sutiles en el código generado por IA) sigue siendo alto; con 41 h/semana su probabilidad depende directamente de R-17, porque la revisión humana es la mitigación principal.

## 2. Registro completo

| ID | Riesgo | P | I | Mitigación | Disparador | Plan alternativo | Tareas |
|---|---|---|---|---|---|---|---|
| R-01 | Guardado condicionado no atómico en POSIX local: ventana TOCTOU residual; etag local probablemente basado en mtime; posibles caídas de Gio a escritura no atómica o pérdida de permisos/propietario | Alta | Alta | Spike temprano; hash + etag justo antes; verificación posterior; capacidades honestas; matriz de pruebas | Resultado del spike muestra sobrescritura sin detección o truncado posible | Bloquear guardado normal en esos casos y ofrecer Guardar como; documentar limitación (D-12) | ENV-T025, ENV-T028, ENV-T030, ENV-T067 |
| R-02 | Flatpak: selección de carpeta por portal no persiste tras reinicio o no permite enumerar/monitorizar | Media | Alta | Spike en S03, siete sprints antes del catálogo (S10) | El prototipo pierde acceso tras reiniciar o no enumera | D-19: permiso ampliado justificado o Workspace limitado en Flatpak con aviso; release nativo no afectado | ENV-T054, ENV-T055, ENV-T077, ENV-T082 |
| R-03 | Interrupciones o ausencias no planificadas (enfermedad, vacaciones de Navidad más largas que un día, otros compromisos) | Media | Alta | D-03 confirmado; reserva del 20 %; holgura acumulada de 61,88 h; WIP = 1 | Semana con menos del 80 % de la capacidad prevista por una ausencia | Replanificar fechas (no recortar calidad); renegociar alcance solo vía PRD | Todas |
| R-04 | Parser/ValueCodec más complejos de lo estimado (escapes, cierres multilínea, comentarios finales) | Media | Alta | ENV-T006 antes del código; fixtures y proptest; ante la duda, `Raw` | Una tarea de E2 supera su estimación en > 50 % | Reducir el subconjunto editable (más casos `Raw`), con aprobación (D-11) | ENV-T006–T014, ENV-T016 |
| R-05 | Curva de gtk4-rs/libadwaita: subclases, modelos de lista, asincronía; rendimiento con muchas filas | Media | Media | Spike ENV-T001; EditorView temprano en fase C (S07); medición ENV-T074 (S11) | EditorView o edición superan estimación > 50 %; render lento con 1 000 líneas | Cambiar a lista virtualizada (`GtkListView`) para documentos grandes; documentar límites | ENV-T001, ENV-T033, ENV-T036, ENV-T074 |
| R-06 | Incompatibilidad de versiones entre gtk-rs, libadwaita del runtime y distribuciones nativas | Media | Media | ENV-T001 fija versiones y runtime; Cargo.lock reproducible | Build falla en el SDK Flatpak o en la distro de prueba | Ajustar mínimo de libadwaita (D-05) o runtime (D-06) | ENV-T001, ENV-T033, ENV-T077 |
| R-07 | Fuga de secretos a logs, panics, mensajes de error, Debug o configuración | Media | Alta | ENV-T004 desde S01; centinelas en fixtures; ENV-T070 y repetición con UI en ENV-T075; revisión de seguridad | Centinela detectado en cualquier salida | Bloquear merge; corrección inmediata y test de regresión | ENV-T004, ENV-T015, ENV-T055, ENV-T070, ENV-T075 |
| R-08 | Decisiones pendientes (sintaxis, app ID, licencia, permisos) resueltas tarde, causando retrabajo. Agravado en el plan v2: las fechas límite se adelantan y varias caen en festivos | Alta | Media | Puertas de aprobación (P-ADR-06); fechas límite en MILESTONES §2; pedir las decisiones al entregar cada spike, no al inicio del sprint dependiente | Decisión no tomada en su fecha límite | Implementar la recomendación por defecto y asumir retrabajo explícito | D-07, D-08, D-11, D-12, D-19 |
| R-09 | Código generado por IA aparentemente correcto con errores sutiles (offsets de bytes, EOL, estados) | Alta | Media | DoD exige tests y revisión humana; arnés de fidelidad; proptest; ningún "terminado" sin evidencia | Defectos de regresión en fidelidad o conflictos | Aumentar cobertura de propiedades; congelar cambios hasta estabilizar | E2, E3, E4 |
| R-10 | Requisitos de Flathub (icono, metainfo, permisos, verificación del app ID) y tiempo de revisión | Media | Media | ENV-T084 antes del RC; D-22 separa release de aceptación en Flathub | Lint con errores no triviales | Publicar v0.1.0 fuera de Flathub y completar el envío después | ENV-T078, ENV-T079, ENV-T084, ENV-T087 |
| R-11 | Pruebas GTK en CI sin display reportadas como aprobadas | Media | Media | ENV-T005 con display virtual; omitidas explícitas | Job verde sin ejecutar pruebas GTK | Ejecutar pruebas GTK localmente con evidencia antes de cada hito | ENV-T005 |
| R-12 | Asociación MIME o CLI en Flatpak no funcionan como se espera | Media | Baja | Comprobación preliminar en ENV-T054; validación en ENV-T081 | Abrir desde Files o `flatpak run` falla | Documentar "Abrir con" y el uso de `flatpak run` como limitación | ENV-T054, ENV-T081 |
| R-13 | Diseño del icono fuera de las habilidades disponibles | Media | Baja | Timebox de 5 h; plantillas de la HIG | Icono no legible a 16 px | Icono geométrico simple; mejora posterior | ENV-T079 |
| R-14 | Pérdida de cambios al cerrar ventanas, salir o abrir varios archivos | Baja | Alta | ENV-T035, ENV-T044 y guion manual | Cualquier caso de pérdida en pruebas | Bloqueo de release (criterio no-release) | ENV-T035, ENV-T044, ENV-T060 |
| R-15 | Eventos de FileMonitor inconsistentes (editores que guardan por renombrado, eventos duplicados o ausentes) | Alta | Alta | Verificación por hash, coalescencia, rearmado del monitor, revalidación al guardar | Cambio externo no detectado en ENV-T069 | Comprobación periódica ligera mientras la ventana tiene foco (propuesta, requiere aprobación) | ENV-T031, ENV-T069 |
| R-16 | Agotamiento de la holgura en fases tardías (defectos acumulados de UI) | Media | Media | Buffers explícitos (ENV-T053, ENV-T063, ENV-T075, ENV-T086, ENV-T088) | Buffer consumido antes de terminar el sprint | Desplazar hitos una semana tras informar; nunca omitir regresión | E8 |
| R-17 | **Sostenibilidad de 41 h/semana con un único desarrollador**: 41 h es más de una jornada completa de trabajo dedicada al proyecto durante 15 semanas seguidas, incluidas las de Navidad. Riesgo de fatiga, de revisiones superficiales del código generado por IA (refuerza R-09) y de caída brusca del rendimiento a mitad de proyecto | Alta | Alta | (1) Mantener la reserva del 20 % (D-35) y no usarla para meter tareas; (2) WIP = 1 y límite de 8 h por tarea; (3) registro semanal de horas reales y de defectos encontrados en revisión; (4) las tareas P0 de E/S y fidelidad se revisan con descanso de por medio, nunca al final de una jornada larga; (5) holgura de S14–S15 (36,6 h) como colchón final | Dos semanas seguidas por debajo del 80 % de lo planificado, o aumento de defectos detectados en QA tras una revisión aprobada | Replanificar con la capacidad real observada (p. ej. 30 h brutas/24 h netas => release ~principios de abril de 2027); nunca recortar pruebas ni revisión de seguridad para mantener la fecha | Todas |

## 3. Riesgos de la ruta crítica y su validación temprana

| Elemento de ruta crítica | Riesgo | Validación temprana |
|---|---|---|
| Especificación → parser → codec (S01–S03) | R-04, R-08 | ENV-T006 y fixtures en S01, proptest en S02; D-11 antes del 2026-11-30 |
| Guardado seguro (S04–S05) | R-01 | Spike ENV-T025 en S02, tres sprints antes de ENV-T028 |
| Workspace en Flatpak (S10–S13) | R-02 | Spike ENV-T054 en S03, siete sprints antes del catálogo |
| UI de edición (S07–S10) | R-05, R-06 | Spike ENV-T001 en S01; ENV-T039 (S07) antes de ENV-T037 (S08) |
| Monitor de cambios (S05–S06) | R-15 | Pruebas con escritores por renombrado en ENV-T031/ENV-T069 |
