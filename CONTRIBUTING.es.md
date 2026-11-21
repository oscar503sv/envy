# Contribuir a Envy

[English](CONTRIBUTING.md) · **Español**

¡Gracias por tu interés en Envy! El proyecto está en una fase temprana: ya existen los requisitos, el diseño técnico y el plan de v0.1.0 (ver [`docs/`](docs/)), y el código se construye sprint a sprint. Hasta que la base esté lista, las contribuciones más útiles son comentarios sobre los documentos de diseño y reportes de bugs o ideas bien descritos.

## Reglas básicas

- **Nunca compartas secretos reales.** No pegues contenido real de archivos `.env`, tokens, contraseñas ni claves privadas en issues, pull requests, logs, capturas de pantalla ni fixtures de prueba. Usa valores claramente falsos, como `EXAMPLE_TOKEN_123`.
- Sé respetuoso. Envy sigue el [Código de Conducta de GNOME](https://conduct.gnome.org/).
- Ten en cuenta el alcance. Envy es un editor local de archivos dotenv. Funciones como la sincronización en la nube, los proveedores de secretos remotos, la ejecución de comandos o la expansión de variables quedan fuera de los objetivos de forma explícita (ver PRD §2.2).

## Reportar bugs

Abre un issue con:

- Qué hiciste, qué esperabas y qué ocurrió en su lugar.
- Tu distribución, versión de GNOME y cómo instalaste Envy (Flatpak o nativo).
- Un `.env` mínimo **inventado** que reproduzca el problema, si el bug afecta al análisis o al guardado. Describe los saltos de línea, el BOM, las comillas y los permisos cuando sean relevantes.

Si Envy perdió o corrompió datos, indícalo en el título. Esos bugs tienen la máxima prioridad.

## Reportar problemas de seguridad

**No** abras un issue público para vulnerabilidades, por ejemplo fugas de secretos en logs, manejo inseguro de archivos o ataques con enlaces simbólicos. Repórtalas en privado mediante los [avisos de seguridad de GitHub](https://github.com/oscar503sv/envy/security/advisories/new).

## Proponer cambios

1. Para cualquier cosa que vaya más allá de un arreglo pequeño, abre primero un issue para acordar el enfoque antes de escribir código.
2. Haz un fork del repositorio y crea una rama desde `main` (por ejemplo `feat/paste-preview` o `fix/crlf-insert`).
3. Mantén cada pull request centrado en un solo cambio y referencia el issue o la tarea de planificación que resuelve (por ejemplo `ENV-T016`).
4. Asegúrate de que pasan todas las comprobaciones de abajo antes de pedir revisión.

## Pautas de desarrollo

### Código

- El código, los identificadores y los comentarios se escriben en inglés.
- Formatea con `cargo fmt` y mantén limpio `cargo clippy --all-targets -- -D warnings`.
- Escribe comentarios solo para explicar el *porqué*, no el *qué*.
- Sigue la arquitectura de [`docs/TECHNICAL_DESIGN.md`](docs/TECHNICAL_DESIGN.md). El núcleo de dominio (parser, modelo de documento, comandos de edición) no debe depender de GTK.
- No añadas una dependencia sin discutirlo antes. Comprueba si ya existe algo equivalente en el proyecto o en la biblioteca estándar.

### Seguridad de archivos y privacidad

Estas reglas no son negociables, y las revisiones las comprueban explícitamente:

- Los bytes no modificados deben sobrevivir exactamente a un ciclo de apertura y guardado. Nunca normalices los saltos de línea, recortes espacios ni reconstruyas el archivo a partir de un mapa de claves.
- Nunca escribas valores de variables en logs, mensajes de error, salidas `Debug`, panics ni archivos de configuración.
- Nunca ejecutes ni expandas el contenido de los archivos.
- Nunca informes de un guardado como exitoso si no se ha verificado, y nunca sobrescribas en silencio un archivo modificado externamente.

### Pruebas

- Cada corrección de un bug incluye una prueba que lo reproduce.
- Los cambios en el parser y en la edición necesitan pruebas basadas en fixtures, incluidos CRLF, saltos de línea mixtos, BOM, ausencia de newline final, valores multilínea y sintaxis no admitida. Las pruebas basadas en propiedades son bienvenidas.
- El comportamiento con el sistema de archivos se prueba contra archivos temporales reales, no con mocks.
- Ejecuta la suite completa con `cargo test` antes de abrir un pull request. Las pruebas de GTK que necesitan pantalla deben aparecer como omitidas cuando no hay pantalla disponible, nunca como aprobadas.

### Commits

Usa [Conventional Commits](https://www.conventionalcommits.org/es/):

```
feat(parser): support export prefix
fix(storage): keep file mode after save
docs: describe CLI usage under Flatpak
```

Los tipos habituales son `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci` y `chore`. Escribe el asunto en modo imperativo y en inglés.

### Interfaz de usuario

- Sigue las [Pautas de Interfaz Humana de GNOME](https://developer.gnome.org/hig/) y usa widgets de libadwaita.
- Toda acción debe poder realizarse solo con el teclado, con el foco visible.
- Nunca transmitas información solo mediante el color, y da nombres accesibles a los botones que solo muestran un icono.
- Marca para traducción todas las cadenas visibles para el usuario.

## Licencia

Al contribuir, aceptas que tus contribuciones se publiquen bajo la [Licencia Pública General de GNU v3.0 o posterior](LICENSE), la misma licencia del proyecto.
