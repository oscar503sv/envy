# Envy — Plantilla de ejecución de tareas para agentes

> Plantilla reutilizable para delegar **una** tarea de [BACKLOG.md](BACKLOG.md) a agentes (Explorador de código, Implementador, QA Engineer, Security Reviewer) sin perder el contexto del PRD y del diseño técnico.  
> Uso: copiar la §2, rellenar los campos `{…}` con los datos de la tarea y entregar el bloque al agente correspondiente. Solo se ejecutan tareas en estado `Ready` ([DEFINITION_OF_DONE.md §1](DEFINITION_OF_DONE.md#1-definition-of-ready-tarea-preparada)) y con autorización del usuario.

## 1. Flujo por tarea

```text
Ready ──► 1. Explorador (contexto y plan) ──► 2. Implementador (código + tests)
      ──► 3. QA Engineer (verificación independiente)
      ──► 4. Security Reviewer (si la tarea lo exige)
      ──► 5. Revisión humana del diff + evidencia ──► Done
```

- Cada fase entrega un **informe de traspaso** (§3) que la siguiente recibe junto con este paquete.
- Si una fase detecta que la tarea contradice PRD/TD o requiere una decisión nueva, **se detiene** y lo registra como pendiente en DECISIONS.md; no decide por su cuenta.
- WIP = 1: no se empieza otra tarea de implementación mientras esta no esté en `Done` o bloqueada.

## 2. Paquete de tarea (copiar y rellenar)

```markdown
# Tarea {ENV-T###} — {Título}

## Contexto obligatorio (leer antes de actuar)
- Producto: Envy v0.1.0, editor dotenv nativo GNOME (Rust + GTK4 + libadwaita), local-first, sin red ni telemetría.
- Fuente de alcance: docs/PRD.md — secciones: {p. ej. §5 ENV-002, §6 US-02, §8}
- Diseño técnico: docs/TECHNICAL_DESIGN.md — secciones: {p. ej. §5.2 invariantes, §5.3 sintaxis, §6.1 comandos}
- Especificación de sintaxis (si aplica): docs/SYNTAX.md — reglas: {…}
- Decisiones aplicables (docs/planning/DECISIONS.md): {D-xx: estado (aprobada / recomendación por defecto aceptada)}
- Reglas no negociables:
  1. Nunca reconstruir el documento desde un HashMap; mutaciones solo con comandos tipados (TD §1).
  2. Bytes no modificados se preservan exactamente (PRD US-02).
  3. Ningún valor ni contenido de archivo en logs, errores, Debug, panics, configuración o salida de tests (PRD §8.2).
  4. Nunca declarar éxito de guardado sin confirmación; nunca sobrescribir silenciosamente (PRD §8.7).
  5. No editar enlaces simbólicos por defecto; no crear respaldos (PRD §8.4–8.5).
  6. No ejecutar ni expandir contenido dotenv (PRD §8.6).
  7. Núcleo (domain/) sin dependencias GTK (ADR-002).
  8. No añadir funcionalidades fuera de esta tarea ni dependencias sin aprobación.

## Datos de la tarea (copiados del backlog)
- Épica: {E#} · Prioridad: {P#} · Estimación: {h} h · Sprint: {S##}
- Origen: {requisitos/decisiones}
- Depende de (en Done): {IDs}
- Descripción: {…}
- Entregables / archivos previstos: {…}
- Criterios de aceptación:
  - [ ] {…}
- Pruebas exigidas: {…}
- Riesgos / seguridad: {…}
- Security Reviewer obligatorio: {sí/no}

## Fuera de alcance de esta tarea
- {lo que NO se debe tocar, p. ej. UI, otras épicas, refactors no pedidos}

## Comandos de verificación
- cargo fmt --check
- cargo clippy --all-targets -- -D warnings
- cargo test
- {comandos específicos: p. ej. cargo test --test fidelity}

## Formato de entrega
Informe de traspaso según docs/planning/TASK_TEMPLATE.md §3.
```

## 3. Informe de traspaso (cada agente)

```markdown
## Informe {rol} — {ENV-T###}
- Resultado: {completado / bloqueado / requiere decisión}
- Resumen (≤ 5 líneas):
- Archivos creados/modificados:
- Evidencia: {comandos ejecutados y resultado; tests añadidos; capturas}
- Criterios de aceptación: {lista con ✔/✘ y evidencia por criterio}
- Desviaciones respecto al PRD/TD o a la tarea: {ninguna / detalle}
- Decisiones nuevas necesarias: {ninguna / D-xx propuesta}
- Riesgos detectados:
- Horas reales:
```

## 4. Instrucciones específicas por rol

### 4.1 Explorador de código
- **Objetivo:** reunir el contexto mínimo para implementar sin suposiciones.
- **Hace:** localiza módulos, tipos y tests afectados; identifica patrones existentes a reutilizar; confirma que las dependencias de la tarea están realmente terminadas (evidencia, no presencia de archivos); propone un plan de cambios por archivo y los tests a escribir.
- **No hace:** modificar archivos; ampliar el alcance.
- **Entrega:** plan de implementación (archivos, funciones, tests) + riesgos + preguntas abiertas.

### 4.2 Implementador
- **Objetivo:** cumplir los criterios de aceptación con el cambio mínimo.
- **Hace:** escribe primero o a la vez las pruebas que fijan el comportamiento; implementa; ejecuta los comandos de verificación; documenta el "por qué" solo cuando no es evidente; respeta convenciones del proyecto (código e identificadores en inglés).
- **No hace:** añadir dependencias, cambiar arquitectura, tocar otras tareas, hacer commits o push sin autorización del usuario.
- **Entrega:** diff, salida de los comandos y el informe de traspaso.

### 4.3 QA Engineer
- **Objetivo:** verificar de forma independiente que cada criterio de aceptación se cumple.
- **Hace:** revisa los tests (¿prueban comportamiento y casos límite/errores?); ejecuta la suite; para UI, ejecuta los casos del guion manual (docs/testing/MANUAL_TESTS.md) con evidencia; comprueba que la fidelidad sigue al 100 %; intenta romper la funcionalidad (entradas raras, EOL mixtos, Unicode, duplicados, archivos cambiando durante la operación).
- **No hace:** dar por buena una tarea por la existencia de archivos o por la afirmación del implementador.
- **Entrega:** veredicto por criterio con evidencia; defectos con pasos de reproducción.

### 4.4 Security Reviewer
- **Objetivo:** confirmar que la tarea no introduce pérdida de datos ni filtración de secretos.
- **Revisa como mínimo:**
  - [ ] Ningún valor/contenido en `tracing`, errores (`Display`/`Debug`), panics, configuración, mensajes de UI no solicitados o salida de tests (buscar el centinela).
  - [ ] Guardado: comprobación de versión antes de escribir, sin éxito sin verificación, sin respaldos, sin truncado ante error.
  - [ ] Enlaces simbólicos y archivos especiales rechazados por defecto.
  - [ ] Permisos (p. ej. 0600) conservados o error visible.
  - [ ] Contenido pegado/abierto tratado como datos (sin expansión ni ejecución).
  - [ ] Portapapeles solo por acción explícita.
  - [ ] Flatpak: ningún permiso ampliado sin justificación aprobada.
- **Entrega:** aprobado / cambios requeridos, con referencias a líneas concretas.

## 5. Ejemplo rellenado (resumen)

> **ENV-T012 — Implementar el serializador puro y el arnés de fidelidad byte a byte.** PRD §5 ENV-001, §10, §14; TD §5.2 (invariante 7), §5.4, §14.1. Depende de ENV-T010, ENV-T011, ENV-T007 (en Done con evidencia). Criterios: 100 % de fixtures idénticos; mensajes de fallo solo con offset/longitud; sin normalización de BOM/EOL. Fuera de alcance: edición, UI, E/S real. Verificación: `cargo test --test fidelity` + test negativo del formato del mensaje. Security Reviewer: no obligatorio (QA comprueba que los fallos no imprimen bytes).
