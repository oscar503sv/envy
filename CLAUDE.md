# Envy: guía de trabajo para agentes

Editor nativo de GNOME para archivos `.env`, en Rust (GTK4/libadwaita). Este archivo describe cómo se desarrolla Envy con agentes de IA para poder retomar el trabajo desde cualquier equipo. La fuente de verdad del plan es `docs/planning/`.

## Fuentes de verdad

- Alcance: `docs/PRD.md`. Diseño: `docs/TECHNICAL_DESIGN.md`. Sintaxis: `docs/SYNTAX.md`.
- Plan: `docs/planning/BACKLOG.md` (tareas `ENV-T###`, estado y criterios), `SPRINT_PLAN.md` (sprints y progreso real), `DECISIONS.md` (decisiones `D-xx`; lo resuelto por el usuario manda), `RISKS.md`, `MILESTONES.md`, `DEFINITION_OF_DONE.md`.
- Pruebas de guardado seguro: `docs/testing/SAVE_SAFETY_MATRIX.md`.
- Seguimiento: el backlog está duplicado en Orbit y Linear. Los identificadores, estados y URL de esas herramientas son privados y **no** se guardan en este repositorio público: están en la memoria local del agente o se piden al usuario. Verifica siempre el ID de la tarea antes de cambiar su estado; en Linear los números no siguen el orden de `ENV-T###`.

## Roles y modelos

- **Orquestador** (modelo principal, Opus): planifica, redacta los briefs, revisa todo lo que entregan los subagentes, ejecuta las comprobaciones y habla con el usuario. Nunca da por buena una entrega sin revisar el diff y ejecutar las pruebas.
- **Implementación** (dos modelos, ambos vía `delegate_task` con rol `implementation` y en modo asíncrono):
  - GPT-6.2-Sol (proveedor `codex`, modelo `gpt-6.2-sol`). No tiene red: si hacen falta crates nuevos, descárgalos tú antes (`cargo fetch`).
  - DeepSeek V4.1 Flash por OpenCode Go (proveedor `opencode`, modelo `opencode-go/deepseek-v4.1-flash`).
  - **Árbol de trabajo:** las tareas pequeñas o seguidas se hacen en el árbol principal, una detrás de otra. Las tareas que corren de verdad en paralelo (por ejemplo, una por modelo) van cada una en su propio worktree (`delegate_task` con `isolation: "worktree"`). Así ningún agente pisa ni compila el trabajo a medias de otro. El orquestador revisa cada rama por separado, ejecuta las comprobaciones sobre ella y la integra en `main` con permiso del usuario. Si dos agentes comparten el árbol por excepción, el brief les asigna archivos distintos.
- **QA**: GLM-5.3 vía `delegate_task` (proveedor `opencode`, modelo `opencode-go/glm-5.3`, rol `test`), en solo lectura y sobre una copia del repo.
- **Seguridad**: subagente nativo `cybersecurity-engineer` en primer plano. Obligatorio cuando la tarea toca archivos, datos sensibles, validación de entrada o configuración de seguridad, o cuando el backlog lo lista en «Agentes».
- Cada revisión o re-verificación es un `delegate_task` nuevo; no reutilices un hilo ya terminado. Los hallazgos pequeños los corrige el orquestador directamente.

## Flujo de cada tarea

1. Marca la tarea `In Progress` en Orbit y Linear, y en el campo «Estado» de `BACKLOG.md`.
2. Lee la ficha de la tarea completa, incluidas las notas «Heredado de…» y «Filas añadidas…», y las decisiones que cita.
3. Si hay una decisión que no está resuelta, pregúntala al usuario con opciones y una recomendación, y regístrala en `DECISIONS.md`.
4. Delega la implementación con un brief cerrado: alcance, reglas del proyecto, pruebas esperadas y comandos de verificación.
5. Revisa el diff y ejecuta tú las comprobaciones (abajo). No te fíes de que el subagente diga que pasan.
6. Lanza QA y seguridad en paralelo. Consolida los hallazgos, verifica los graves y corrige los que bloquean. Vuelve a ejecutar las pruebas.
7. Anota en `BACKLOG.md` lo que queda para otras tareas («Heredado de ENV-T0xx…» o «Filas añadidas…»). Las notas se insertan justo **antes** del ancla `<a id="env-tNNN">` de la tarea **siguiente**.
8. Pide permiso al usuario para el commit. Con permiso: commits con conventional commits, en inglés y **sin coautores**; push a `main`; espera el CI (`gh run watch <id> --exit-status` en segundo plano).
9. Con el CI en verde: marca la tarea `Done` en Orbit, Linear y `BACKLOG.md` (estado y casillas de criterios), y actualiza el progreso de `SPRINT_PLAN.md` al cerrar un sprint.

## Comprobaciones obligatorias

```sh
cargo fmt --check
cargo clippy --all-targets --all-features --locked -- -D warnings
ENVY_REQUIRE_FIXTURES=acl,selinux cargo test --locked --all-targets   # en Fedora; en CI solo acl
cargo test --locked --doc
```

En pruebas sensibles a carreras (por ejemplo `tests/save_safety.rs`, `tests/gio_storage.rs`), ejecútalas tres veces seguidas para descartar fallos intermitentes. Nunca digas que algo pasa sin haberlo ejecutado.

## Reglas del proyecto

- **Arquitectura (ADR-002):** nada de GTK, GLib ni GIO en `domain` ni en `services`; lo comprueba `tests/architecture.rs`.
- **Código de producción:** Clippy prohíbe `unwrap`, `expect`, indexación y cortes de `str`. `unsafe_code = "deny"`: cada bloque `unsafe` lleva su `#[allow(unsafe_code, reason = …)]` y un comentario `// SAFETY:`. En los auxiliares de pruebas se usa `assert!` en lugar de `unwrap`.
- **Privacidad (PRD §8.2):** claves, valores, contenido y rutas nunca aparecen en logs, `Debug`, `Display`, panics, errores ni salida de pruebas. En las pruebas usa el centinela `ENVY_TEST_SENTINEL_9F2C` (`test_support::SENTINEL`) y compara contenido con `assert!(a == b)`, nunca con `assert_eq!`.
- **No modifiques** `tests/fixtures/` (tienen checksums verificados en CI).
- **Comentarios** solo para explicar el «por qué». Identificadores y commits en inglés; documentación de planificación en español.
- **Dependencias:** antes de añadir una, comprueba si ya hay algo equivalente; no subas versiones mayores sin permiso del usuario. `cargo deny` corre en CI.
- **Nunca** hagas commit ni push sin permiso explícito del usuario.

## Entorno

- Toolchain fijado en `rust-toolchain.toml` (1.95.0). Si cargo responde «unknown proxy name», antepón `$HOME/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin` al `PATH`.
- Dependencias de sistema: archivos de desarrollo de GLib/Gio (`glib2-devel` en Fedora, `libglib2.0-dev` en Debian/Ubuntu) y `acl` para las pruebas de ACL.
- `/tmp` suele ser un tmpfs pequeño. Los subagentes deben usar `$HOME/.cache` para `CARGO_TARGET_DIR` y directorios temporales, y borrarlos al terminar; si `/tmp` se llena, el enlazador falla con «Bus error».
