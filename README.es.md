# Envy

[English](README.md) · **Español**

> *Your environments, beautifully organized.*

Envy es un editor de archivos dotenv (`.env`) local-first y nativo de GNOME. Abre un archivo y cambia una variable en segundos, o ten a un clic los archivos de entorno de todos tus proyectos, sin abrir un IDE y sin que Envy reformatee tu archivo.

> **Estado: pre-alpha, núcleo en desarrollo.** Todavía no hay una aplicación utilizable: el motor de documentos y el almacenamiento seguro de archivos están implementados y probados; la interfaz GTK aún no ha empezado. Los requisitos de producto, el diseño técnico y el plan de entrega de v0.1.0 están en [`docs/`](docs/). La primera versión, v0.1.0, está estimada para marzo de 2027.

## Por qué Envy

Cambiar una sola variable no debería requerir un IDE. Los editores de texto generales no ofrecen una vista estructurada de las asignaciones, no protegen los secretos en pantalla, no organizan los archivos por proyecto ni avisan cuando otro proceso (un script, la herramienta de un compañero o un agente de IA) modifica el archivo que estás editando.

Envy pretende hacer exactamente eso y nada más: **no** es un gestor de secretos remoto, ni un IDE, ni una herramienta de despliegue.

## Funciones previstas (v0.1.0)

**Quick Edit**: `envy .env` desde una terminal, o abre cualquier archivo desde el selector.

- Vista estructurada de las asignaciones, con búsqueda por clave.
- Edición de valores sin comillas, con comillas simples, con comillas dobles y multilínea.
- Añadir y eliminar claves, pegar bloques de líneas `KEY=value` con previsualización, y copiar un valor o el documento completo.
- Deshacer y rehacer.
- Los valores que parecen secretos (contraseñas, tokens, claves) se ocultan en pantalla hasta que los revelas.

**Workspace**: registra carpetas de proyectos locales y sus subcarpetas de servicio (`backend/`, `frontend/`…).

- Ve todos los archivos `.env`, `.env.*` y `*.env` de un proyecto sin recorrer el sistema de archivos.
- Etiquetas de entorno (desarrollo, staging, producción) mostradas como texto, no solo con color.
- Compara un archivo con su `.env.example` y añade las claves que faltan.
- Avisos de claves duplicadas.

**Seguridad de archivos**, en ambos modos:

- **Sin pérdidas:** abrir y guardar sin cambios produce exactamente los mismos bytes. Editar un valor cambia solo ese valor, nunca los comentarios, las comillas, los espacios, los saltos de línea ni el BOM.
- **Cambios externos:** si el archivo cambia en disco y no tienes ediciones locales, Envy lo recarga. Si tienes ediciones locales, Envy muestra un conflicto y nunca descarta ninguna de las dos versiones sin una decisión explícita tuya.
- **Guardado seguro:** reemplazo condicional con verificación posterior, permisos conservados (p. ej. `0600`), sin sobrescrituras silenciosas y sin copias de respaldo de tus secretos.
- **Enlaces simbólicos:** nunca se reemplazan por archivos regulares.

## Privacidad

- Todo permanece en tu equipo: sin cuentas, sin nube, sin sincronización y sin telemetría.
- Los valores de las variables nunca se escriben en logs, mensajes de error ni en la configuración de Envy. El catálogo de proyectos solo guarda rutas, nombres y etiquetas.
- Envy nunca ejecuta ni expande nada: `$VAR`, `${VAR}` y `$(command)` se muestran como texto literal.
- Solo se copia al portapapeles cuando lo pides. Ten en cuenta que el portapapeles puede conservar secretos.

## Sintaxis admitida

Envy admite un subconjunto documentado de dotenv:

- `KEY=value`, `KEY='value'`, `KEY="value"` y valores entre comillas que ocupan varias líneas.
- Prefijo `export` opcional.
- Líneas de comentario (`# …`) y comentarios al final (` # …`, precedidos de un espacio).
- Las comillas dobles reconocen `\\`, `\"`, `\n`, `\r` y `\t`. Las comillas simples son totalmente literales.

Todo lo que queda fuera del subconjunto se muestra en solo lectura como "sintaxis no editable" y se conserva byte a byte. Las reglas completas están en [`docs/SYNTAX.md`](docs/SYNTAX.md).

## Instalación

Todavía no está disponible. v0.1.0 se distribuirá como Flatpak y se enviará a Flathub después del lanzamiento. También se admitirán compilaciones nativas.

## Compilar desde el código fuente

Envy está en una fase temprana: el crate compila, pero todavía no hay ventana de aplicación.

Stack:

- Rust 1.95 o superior, edición 2024 (`rust-toolchain.toml` fija el toolchain para `rustup`)
- GTK 4 y libadwaita 1.5 o superior (a partir del hito de interfaz)
- Gio para el acceso y la monitorización de archivos; el guardado usa escritura por descriptores con `libc` (rename atómico con verificación posterior)
- Cargo con `build.rs` como sistema de build
- Flatpak con el runtime GNOME 51

GitHub Actions ejecuta las comprobaciones del núcleo en Linux con los archivos de desarrollo de GLib, pero sin GTK. En local necesitas los archivos de desarrollo de GLib/Gio (`libglib2.0-dev` en Debian/Ubuntu, `glib2-devel` en Fedora). Las pruebas de ACL usan `setfacl` del paquete `acl` y se omiten si falta, salvo que `ENVY_REQUIRE_FIXTURES=acl` (como en CI) convierta la omisión en fallo. Usa los comandos siguientes para ejecutar las mismas comprobaciones en local. La comprobación de dependencias requiere cargo-deny.

```sh
cargo build
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo test --locked --doc
cargo deny check
```

Las instrucciones para compilar el Flatpak se añadirán junto con el manifiesto.

## Documentación

- [Requisitos de producto (PRD)](docs/PRD.md)
- [Diseño técnico](docs/TECHNICAL_DESIGN.md)
- [Sintaxis dotenv admitida](docs/SYNTAX.md)
- [Matriz de pruebas de guardado seguro](docs/testing/SAVE_SAFETY_MATRIX.md)
- [Planificación de v0.1.0](docs/planning/): roadmap, plan de sprints, backlog, hitos, riesgos, decisiones y definición de terminado

## Contribuir

Las contribuciones serán bienvenidas en cuanto esté lista la base del proyecto. Lee primero [CONTRIBUTING.es.md](CONTRIBUTING.es.md). **Nunca incluyas secretos reales** en issues, pull requests, capturas de pantalla ni fixtures de prueba.

## Licencia

Envy es software libre, publicado bajo la [Licencia Pública General de GNU v3.0 o posterior](LICENSE) (`GPL-3.0-or-later`).
