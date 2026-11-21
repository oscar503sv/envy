# Envy v0.1.0 — Hitos

> **Estado de todas las fechas:** **estimadas** a partir de [SPRINT_PLAN.md](SPRINT_PLAN.md) (plan v2) con capacidad **confirmada** de 41 h/semana brutas (D-02 resuelta) y reserva del 20 %. **Ninguna es un compromiso aprobado.**  
> Un hito se cumple cuando todos sus criterios de salida tienen evidencia y el usuario lo acepta ([DEFINITION_OF_DONE.md §4](DEFINITION_OF_DONE.md#4-definition-of-done--hito)).

## 1. Resumen

| Hito | Nombre | Fin de sprint | Fecha estimada | Plan v1 (15 h/sem) | Correspondencia |
|---|---|---|---|---|---|
| M1 | Motor dotenv probado | S04 | 2026-12-18 | 2027-01-29 | PRD §11 hito 1 (documento) · TD §15 Hito 1 |
| M2 | Núcleo de persistencia segura y conflictos | S06 | 2027-01-01 (festivo; último día laborable 2026-12-31) | 2027-03-12 | PRD §11 hito 1 (escritura y conflictos) · TD §15 Hito 1 punto 5 |
| M3 | Quick Edit utilizable | S10 | 2027-01-29 | 2027-05-14 | PRD §11 hito 2 · TD §15 Hito 2 |
| M4 | Workspace funcional | S12 | 2027-02-12 | 2027-06-25 | PRD §11 hito 3 · TD §15 Hito 3 |
| M5 | MVP completo (funcionalidad + endurecimiento + Flatpak validado) | S14 (tras ENV-T075) | 2027-02-26 | 2027-07-30 | PRD §11 hito 4 (parte) |
| M6 | Release candidate v0.1.0-rc | S14 | 2027-02-26 | 2027-08-06 | PRD §11 hito 4 |
| M7 | v0.1.0 publicada | S15 | 2027-03-05 | 2027-08-13 | Criterio de salida PRD §11 |

- M2 es un hito intermedio añadido: separa el motor de documentos de la persistencia segura (los dos bloques P0 con más riesgo) antes de la UI.
- Con 41 h/semana, M5 y M6 caen en la misma semana: ENV-T075 (endurecimiento) se ejecuta primero y M5 se verifica antes de generar el RC (ENV-T085). Se deja S15 entera para la regresión del RC y la decisión go/no-go, con 21,8 h de holgura.

## 2. Puertas de decisión (fechas límite)

Regla: la decisión debe estar tomada al inicio del sprint de la primera tarea afectada. Las tareas que dependen de un spike o de la especificación van siempre en un sprint posterior a la tarea que produce la propuesta, para dar tiempo a aprobarla.

| Puerta | Decisión | Propuesta producida por | Antes de | Fecha límite | Plan v1 |
|---|---|---|---|---|---|
| DG-1 | D-01 IDs, D-03 festivos, D-04 prioridades, D-26 orden, D-32 edición de Rust | Este plan / ENV-T001 (S01) | S01 | 2026-11-23 | 2026-11-23 |
| DG-2 | **D-11 sintaxis**, D-23 CI, D-24 proptest | ENV-T006 (S01) | S02 (ENV-T010, ENV-T003, ENV-T013) | **2026-11-30** (ENV-T010 va tercera en S02: margen de 1–2 días) | 2026-12-14 / 2026-12-28 |
| DG-3 | D-25 hash | — | S04 (ENV-T026) | 2026-12-14 | 2027-02-01 |
| DG-4 | **D-12 política de guardado**, D-27 Guardar como, D-34 enlaces duros | ENV-T025 (S02) | S05 (ENV-T028, ENV-T030) | **2026-12-21** (semana festiva) | 2027-02-15 |
| DG-5 | **D-05 libadwaita, D-06 runtime, D-09 build**, D-07 app ID (recomendado), D-15 múltiples documentos, D-28 preferencias, D-33 UTF-8 inválido | ENV-T001 (S01) | S07 (ENV-T033, ENV-T035, ENV-T041) | **2027-01-04** (justo tras festivos) | 2027-03-15 a 2027-03-29 |
| DG-6 | D-13 undo, D-14 sobrescribir | ENV-T039 (S07) | S08 (ENV-T037, ENV-T042) | 2027-01-11 | 2027-03-29 / 2027-04-12 |
| DG-7 | D-16 cierre, D-17 ocultación | — | S09 (ENV-T044, ENV-T048) | 2027-01-18 | 2027-04-12 |
| DG-8 | **D-19 permisos Flatpak**, D-10 configuración, D-29 patrón `.env.*` | ENV-T054 (S03) | S10 (ENV-T055, ENV-T057) | **2027-01-25** | 2027-05-17 / 2027-05-24 |
| DG-9 | D-18 comparación en Quick Edit, **D-07 app ID (obligatorio)** | — | S12 (ENV-T065, ENV-T077) | 2027-02-08 | 2027-05-24 / 2027-06-28 |
| DG-10 | D-20 MIME, D-21 traducciones | ENV-T054 (S03) | S13 (ENV-T080, ENV-T081) | 2027-02-15 | 2027-07-05 |
| DG-11 | D-08 licencia, D-22 definición de release | — | S14 (ENV-T083) | 2027-02-22 | 2027-08-02 |

**Puntos de atención:**
- D-11 tiene solo un fin de semana entre la entrega de ENV-T006 (S01) y su uso (S02). Si no se aprueba a tiempo, se implementa la propuesta y se asume el retrabajo (R-08), o se adelanta trabajo que no depende de ella (ENV-T025, ENV-T003, ENV-T054, ENV-T022).
- DG-4 y DG-5 requieren que el usuario esté disponible para decidir en las semanas festivas.

## 3. Detalle de hitos

### M1 — Motor dotenv probado (estimado 2026-12-18)

- **Entrada:** especificación de sintaxis (ENV-T006) aprobada o aceptada como base (D-11); fixtures creados.
- **Tareas:** ENV-T001–T004, ENV-T006–T020, ENV-T022–T025, ENV-T054 (spikes incluidos). En S04 también se completan ENV-T026, ENV-T027 y ENV-T029, que cuentan para M2.
- **Salida (verificable):**
  - [ ] `serialize(parse(b)) == b` para el 100 % de fixtures UTF-8; proptest sin fallos ni panics.
  - [ ] SetValue/Insert/Remove/Batch con undo/redo exactos en bytes; dirty calculado contra snapshot.
  - [ ] ValueCodec rechaza valores no representables.
  - [ ] Ejemplo `roundtrip` demuestra fidelidad sobre un archivo real sin imprimir valores.
  - [ ] Spikes ENV-T025 y ENV-T054 con conclusiones registradas en DECISIONS.md.
  - [ ] CI del núcleo en verde.
- **Demo:** ejecución de la suite y del ejemplo `roundtrip`.

### M2 — Núcleo de persistencia segura y conflictos (estimado 2027-01-01)

- **Entrada:** M1 cumplido; D-12 decidida.
- **Tareas:** ENV-T021, ENV-T026–T032, ENV-T067–T070.
- **Salida:**
  - [ ] Guardado condicionado: un cambio externo entre lectura y guardado produce conflicto sin escribir.
  - [ ] Éxito solo tras verificación posterior; sin respaldos.
  - [ ] Máquina de estados de TD §7.2 cubierta por tests; nunca se descartan cambios sin evento explícito.
  - [ ] Monitor con coalescencia y detección de escrituras por renombrado.
  - [ ] Matrices de guardado y conflictos en CI; auditoría de fugas del núcleo sin hallazgos.
- **Demo:** script que modifica el archivo durante una edición simulada y muestra conflicto y rechazo de guardado.

### M3 — Quick Edit utilizable (estimado 2027-01-29)

- **Entrada:** M2 cumplido; D-05, D-06, D-09, D-13, D-15 decididas; D-07 recomendada.
- **Tareas:** ENV-T005, ENV-T033–T053.
- **Salida:**
  - [ ] `envy .env` abre, permite buscar, editar (simple y multilínea), crear, eliminar, pegar con previsualización, copiar, deshacer/rehacer y guardar sin registrar proyecto (Situación A).
  - [ ] Cambios externos con y sin cambios locales gestionados sin pérdida (Situación D).
  - [ ] Secretos ocultos por defecto; estados con texto; cierre con cambios protegido; enlaces simbólicos no reemplazados.
  - [ ] Pruebas GTK ejecutadas en CI con display virtual.
  - [ ] Defectos P0 abiertos = 0.
- **Demo:** Situaciones A y D en nativo.

### M4 — Workspace funcional (estimado 2027-02-12)

- **Entrada:** M3 cumplido; D-19, D-10, D-29, D-18 decididas.
- **Tareas:** ENV-T055–T066, ENV-T071, ENV-T074.
- **Salida:**
  - [ ] Agregar/renombrar/quitar proyectos y servicios sin tocar archivos; restauración tras reinicio.
  - [ ] Descubrimiento exacto sin recursión; refresco.
  - [ ] Etiquetas de entorno con texto y corrección persistente; comparación con `.env.example` sin valores; avisos de duplicados.
  - [ ] Navegación adaptable en ventana estrecha.
  - [ ] Situaciones B, C y E completas; catálogo sin valores (centinela).
- **Demo:** Situaciones B, C y E en nativo.

### M5 — MVP completo (estimado 2027-02-26)

- **Entrada:** M4 cumplido; D-07 cerrada; D-20, D-21 decididas.
- **Tareas:** ENV-T072, ENV-T073, ENV-T075, ENV-T077–T082, ENV-T086.
- **Salida:**
  - [ ] Todos los requisitos ENV-001…ENV-020 implementados con evidencia.
  - [ ] Auditoría de accesibilidad y atajos sin hallazgos bloqueantes.
  - [ ] Regresión nativa completa y auditoría de fugas con UI sin hallazgos.
  - [ ] Flatpak construido y flujo TD §12 validado o limitaciones registradas para decisión.
  - [ ] i18n preparada; icono y metadatos validados.
- **Demo:** informe de regresión y recorrido en Flatpak.

### M6 — Release candidate (estimado 2027-02-26)

- **Entrada:** M5 cumplido (verificado dentro de S14 antes de ENV-T083); D-08 y D-22 decididas.
- **Tareas:** ENV-T083, ENV-T084, ENV-T085.
- **Salida:**
  - [ ] README, LICENSE, capturas y limitaciones en el repositorio.
  - [ ] `flatpak-builder-lint` sin errores.
  - [ ] Build RC reproducible desde checkout limpio (nativo y bundle Flatpak).
  - [ ] Checklist de release completo (DEFINITION_OF_DONE §7).
- **Demo:** instalación del RC en entorno limpio.

### M7 — v0.1.0 (estimado 2027-03-05)

- **Entrada:** M6 cumplido.
- **Tareas:** ENV-T076, ENV-T088, ENV-T087.
- **Salida:**
  - [ ] Informe go/no-go con evidencia de cada criterio de release.
  - [ ] Defectos bloqueantes = 0.
  - [ ] **Aprobación explícita del usuario** registrada antes de etiquetar y publicar.
  - [ ] Artefactos publicados corresponden al commit evaluado; envío a Flathub según D-22 (su aceptación no forma parte de esta fecha).
- **Si no-go:** se planifica un ciclo adicional de corrección de una semana y se reevalúa; la nueva fecha se comunica antes de aplicarla.
