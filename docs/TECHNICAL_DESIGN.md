# Envy — Diseño técnico y arquitectura

**Versión del documento:** 1.1 — revisión consolidada  
**Release:** v0.1.0 (MVP)  
**Fecha:** 19 de noviembre de 2026  
**Estado:** Diseño objetivo; no implica que exista código implementado  
**Fuente de alcance:** [`PRD.md`](./PRD.md)

## 1. Resumen de la solución

Envy será una aplicación Linux local-first escrita en Rust y presentada con GTK4 + libadwaita. La arquitectura prioriza un **document engine** desacoplado de GTK que retiene el texto fuente original, representa asignaciones como nodos con rangos de bytes y aplica ediciones mínimas. Una capa de aplicación controla el estado del documento, historial, monitorización y persistencia; la capa gráfica proporciona Quick Edit y Workspace.

**Regla arquitectónica:** la interfaz nunca escribe directamente archivos ni reconstruye documentos mediante un `HashMap`. Todas las mutaciones pasan por comandos tipados y por un único servicio de guardado protegido.

### Flujo alto nivel

```text
Terminal / gestor de archivos        Lanzador GNOME
        |                                  |
        v                                  v
  Quick Edit                          Workspace
        \                                 /
         +------ GTK4 + libadwaita -------+
                         |
                 Application layer
             (DocumentController, Projects)
                         |
                   Domain layer
          (Document, Nodes, Edit Commands)
                         |
              Infrastructure layer
            (Gio files, monitor, config)
                         |
                Archivos locales
```

## 2. Decisiones de arquitectura (ADR resumidos)

| ADR | Decisión | Justificación |
|---|---|---|
| ADR-001 | **Un crate Rust modular** | Evita sobreingeniería; límites por módulos y tests sin GTK |
| ADR-002 | **Core sin dependencias de GTK** | Parser e historial testeables con pruebas unitarias |
| ADR-003 | **Edición basada en fragmentos/rangos originales** | Fidelidad byte a byte al no modificar contenido ajeno |
| ADR-004 | **Comandos de edición tipados** | Undo/redo predecible y validación centralizada |
| ADR-005 | **Un estado `DocumentSession` por archivo abierto** | Historial, versión base y conflictos independientes |
| ADR-006 | **Gio para URI, monitor y operación de guardado** | Compatibilidad con integración GNOME y acceso por portales, sujeto a pruebas |
| ADR-007 | **Configuración sin secretos** | No duplicar valores sensibles en una base de datos o preferencias |
| ADR-008 | **Acceso a carpetas explícito** | Workspace predecible y mínimo privilegio |
| ADR-009 | **Sin parser universal implícito** | Dialectos dotenv incompatibles; los nodos desconocidos se preservan |
| ADR-010 | **Backend de almacenamiento con capacidades** | No atribuir atomicidad, etags ni metadatos a proveedores que no los garantizan |

## 3. Stack recomendado

| Componente | Opción | Observaciones |
|---|---|---|
| Lenguaje | Rust estable, edición 2024 si el toolchain objetivo la soporta | Versionar `Cargo.lock` |
| UI | `gtk4` / `libadwaita` de gtk-rs | Definir versiones **compatibles entre sí** tras iniciar el proyecto |
| API libadwaita mínima objetivo | **1.4** | `AdwNavigationSplitView` y `AdwToolbarView` existen desde 1.4; revisar runtime mínimo en Flatpak |
| Integración sistema | `gio`, `glib` | `GFile`, `FileMonitor`, acciones y aplicación |
| Configuración | `serde` + `toml` **o** GSettings | TOML para catálogo de proyectos; GSettings para preferencias simples |
| Errores | `thiserror` | Errores de dominio semánticos sin imprimir valores |
| Diagnósticos | `tracing` | Filtrado cuidadoso, sin secretos ni contenido de archivos |
| Fixtures y pruebas | `tempfile`, pruebas estándar Rust | `proptest` opcional para propiedades de parser/serializador |
| Hash | SHA-256 o hash criptográfico equivalente | Comparar bytes base/actual, no necesario para almacenar secretos fuera de memoria |

No asumir la versión más reciente de las crates: validar compatibilidad entre GTK del sistema, runtime GNOME elegido y bindings Rust, y fijar el conjunto de dependencias en un primer `Cargo.lock` reproducible.

## 4. Estructura sugerida del repositorio

```text
envy/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── docs/
│   ├── PRD.md
│   └── TECHNICAL_DESIGN.md
├── data/
│   ├── io.github.<owner>.Envy.desktop
│   ├── io.github.<owner>.Envy.metainfo.xml
│   ├── io.github.<owner>.Envy.gschema.xml
│   └── icons/
├── resources/
│   ├── resources.gresource.xml
│   └── ui/
│       ├── window.ui
│       ├── workspace.ui
│       ├── editor.ui
│       └── assignment_row.ui
├── src/
│   ├── main.rs
│   ├── application.rs
│   ├── cli.rs
│   ├── domain/
│   │   ├── mod.rs
│   │   ├── document.rs
│   │   ├── syntax.rs
│   │   ├── edit.rs
│   │   ├── history.rs
│   │   ├── project.rs
│   │   └── compare.rs
│   ├── services/
│   │   ├── mod.rs
│   │   ├── document_service.rs
│   │   ├── project_service.rs
│   │   └── environment_service.rs
│   ├── infrastructure/
│   │   ├── mod.rs
│   │   ├── filesystem.rs
│   │   ├── gio_storage.rs
│   │   ├── file_monitor.rs
│   │   └── project_store.rs
│   └── ui/
│       ├── mod.rs
│       ├── window.rs
│       ├── quick_edit.rs
│       ├── workspace.rs
│       ├── editor.rs
│       ├── assignment_row.rs
│       ├── paste_dialog.rs
│       └── conflict_view.rs
├── tests/
│   ├── fixtures/
│   ├── fidelity.rs
│   ├── editing.rs
│   └── file_conflicts.rs
└── packaging/
    └── flatpak/
```

Los módulos son límites lógicos, no crates independientes. El identificador de aplicación es provisional.

## 5. Diseño del modelo de dominio

### 5.1 Representación esencial

**No usar `HashMap<String, String>` como representación primaria**, porque descarta orden, duplicados, comentarios y estilo de comillas.

```rust
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct EnvDocument {
    /// Snapshot original del archivo en el momento de abrir/recargar.
    pub source: Vec<u8>,
    /// Nodos en orden físico, cada uno cubre un rango del source original.
    pub nodes: Vec<EnvNode>,
    /// IDs estables para filas mientras la sesión esté abierta.
    pub next_node_id: u64,
}

#[derive(Debug, Clone)]
pub struct EnvNode {
    pub id: u64,
    pub source_span: Range<usize>,
    pub kind: NodeKind,
    /// None si el nodo nunca se modificó.
    pub replacement: Option<Vec<u8>>,
    pub removed: bool,
}

#[derive(Debug, Clone)]
pub enum NodeKind {
    Assignment(AssignmentSyntax),
    Comment,
    Blank,
    Raw,
}

#[derive(Debug, Clone)]
pub struct AssignmentSyntax {
    pub key: String,
    pub value_span: Range<usize>,
    pub quote: QuoteStyle,
    pub is_multiline: bool,
    pub is_export: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum QuoteStyle {
    Unquoted,
    Single,
    Double,
}
```

> Esquema conceptual, **no código listo para compilar**. Los rangos `value_span` deben tener semántica consistente (absoluta o relativa), los nodos insertados no tienen span original y la política de editar comillas/escapes todavía requiere pruebas. Elegir definitivamente estas estructuras en el Hito 1.

### 5.2 Invariantes

1. La concatenación de nodos originales sin modificaciones reproduce `source` byte a byte.
2. Un nodo de asignación puede abarcar varias líneas físicas (multilínea), sin dividirse en nodos de texto que invaliden los rangos.
3. Cada `id` identifica una **ocurrencia**, no una clave única.
4. Los nodos `Raw` no pueden recibir operaciones estructuradas de edición.
5. No se reparan ni reformatan regiones ajenas a la operación del usuario.
6. Cambiar el valor de una asignación genera un reemplazo **mínimo** que conserva prefijo/sufijo/terminador si la sintaxis lo permite.
7. El serializador es función pura sobre el documento (`serialize(&EnvDocument) -> Vec<u8>`).
8. El documento abierto se valida UTF-8 antes de ofrecer edición estructurada. El flujo original puede conservar BOM sin normalizarlo.

### 5.3 Especificación de sintaxis v0.1

| Elemento | Admitido | Regla |
|---|---|---|
| `KEY=value` | Sí | Clave estándar ASCII de identificador |
| `KEY = value` | Sí | Conservar espacios alrededor de `=` |
| `export KEY=value` | Sí, subconjunto documentado | `export` se preserva como prefijo textual; no ejecuta shell |
| `KEY='value'` | Sí | Conservar estilo de comillas |
| `KEY="value"` | Sí | Conservar estilo de comillas |
| Valor entre comillas en varias líneas físicas | Sí | Detectar cierre real de comillas con reglas de escape definidas |
| Comentario de línea `# texto` | Sí | Preservar literal |
| Comentario al final `KEY=value # nota` | Sí, cuando el reconocimiento es inequívoco | Conservar sufijo, no incluirlo en el valor editable |
| `KEY=` vacío | Sí | Valor vacío válido |
| Líneas vacías / `LF`, `CRLF`, `CR`, mixtos | Sí | Preservar individualmente |
| `KEY=$OTHER`, `${OTHER}`, `$(command)` | Texto literal | No expandir ni ejecutar |
| Claves con guiones/puntos u otra gramática | `Raw` si quedan fuera del subconjunto | Preservar, no interpretar como asignación estándar |
| Heredoc, shell avanzado, sintaxis no familiar | `Raw` | No se reescribe |
| UTF-8 no válido | Lectura protegida / error de edición | Nunca normalizar ni sobrescribir |

**Cuestión de implementación:** comillas simples y dobles pueden representar escapes de manera diferente entre bibliotecas dotenv. Envy preserva los caracteres originales. Para editar, un `ValueCodec` por estilo define si la transformación es segura; si no, se solicita conversión explícita al usuario o la edición queda bloqueada. Los tests deben fijar reglas verificables, sin simular compatibilidad universal.

### 5.4 Serialización mínima

Pseudocódigo:

```text
serialize(document):
  output = []
  para cada nodo, en orden:
    si removed: continuar
    si replacement existe: output += replacement
    si no: output += source[node.source_span]
  retornar output
```

Los nodos nuevos se insertan con contenido sintético; las terminaciones de línea son parte del fragmento correspondiente. No convertir todas las líneas a `LF` ni aplicar trim global. Los archivos no modificados deben devolverse literalmente, sin siquiera normalizar comentarios o BOM.

## 6. Ediciones e historial (undo/redo)

### 6.1 Comandos

```rust
pub enum EditCommand {
    SetValue {
        node_id: u64,
        before: Vec<u8>,
        after: Vec<u8>,
    },
    InsertAssignment {
        position: usize,
        created_id: u64,
        content: Vec<u8>,
    },
    RemoveAssignment {
        node_id: u64,
        original_position: usize,
        original_content: Vec<u8>,
    },
    Batch(Vec<EditCommand>),
}
```

No almacenar `EditCommand` en preferencias, logs ni discos temporales. Es normal que contenga secretos en memoria. Aunque se utilicen tipos de memoria segura para buffers propios, GTK también puede mantener copias internas; **no prometer borrado criptográfico de secretos en RAM**.

### 6.2 Historial de sesión

```text
apply(cmd): validar -> mutar -> push(undo) -> clear(redo)
undo(): pop(undo) -> aplicar inversa -> push(redo)
redo(): pop(redo) -> reaplicar -> push(undo)
```

Las operaciones de lote son transaccionales: o se aplica el lote entero o nada. Tras guardar, se actualiza el snapshot en disco y el `dirty` se calcula comparando el estado serializado actual con el último snapshot confirmado; no simplemente comprobando si la pila undo tiene elementos.

**Posibles fuentes de confusión:** `Ctrl+Z` dentro de un campo GTK puede activar su undo local. Definir una política única: el historial de Envy controla las ediciones confirmadas de la fila, o el widget integra su propio undo correctamente. No permitir dos historiales contradictorios. Validar con pruebas manuales de foco y atajos.

### 6.3 Pegado de asignaciones

Pipeline: `clipboard text` → parser temporal → validación de bloques y claves → detección de duplicados → diálogo de previsualización → `Batch`.

- Si el bloque incluye líneas no parseables, mostrar su ubicación y opciones seguras, sin ejecutar inserción parcial silenciosa.
- Nunca hacer sustitución automática por nombre si el archivo contiene duplicados.
- Las entradas del lote solo son visibles al confirmar. `Ctrl+Z` revierte todo el lote.

## 7. Estado de documento y sincronización externa

### 7.1 Estado de sesión

```rust
pub struct DocumentSession {
    pub resource: ResourceRef,
    pub document: EnvDocument,
    pub base: DiskVersion,
    pub history: EditHistory,
    pub state: DocumentState,
}

pub enum DocumentState {
    Clean,
    Dirty,
    Conflict,
    ReadOnly,
    Missing,
    Saving,
}

pub struct DiskVersion {
    pub sha256: [u8; 32],
    pub etag: Option<String>,
    pub size: u64,
    pub modified_time: Option<std::time::SystemTime>,
}
```

Tipos conceptuales: `ResourceRef` debe almacenar una URI GFile o abstracción equivalente, evitando asumir siempre rutas POSIX disponibles. Es necesario modelar por separado `dirty` y estado de conflicto en la implementación concreta si deben coexistir; el enum anterior es solo una vista de estados predominantes.

### 7.2 Máquina de estados

| Evento | Sin cambios locales | Con cambios locales |
|---|---|---|
| Evento externo + contenido idéntico | Ignorar | Mantener edición local |
| Evento externo + contenido diferente | Recargar y notificar | Marcar conflicto, mantener contenido local |
| Archivo eliminado/movido | Mostrar `Missing` | Mantener edición local y bloquear guardado normal |
| Guardar con versión de disco igual a `base` | Sin acción | Persistir y actualizar `base` |
| Guardar con versión diferente | Recargar si es apropiado | Bloquear escritura y mostrar conflicto |
| `Recargar y descartar` explícito | Recargar | Descartar historial y ediciones locales solo tras confirmación |
| `Guardar como…` | Guardar copia independiente | Guardar copia independiente sin alterar destino conflictivo |

### 7.3 Diseño del monitor

Usar `Gio.FileMonitor` para recibir eventos `Changed`, `ChangesDoneHint`, `Created`, `Deleted`, `Moved` y otros según backend; **no** asumir que todos se emiten ni que cada evento corresponde a un cambio distinto.

1. Evento del monitor → programar verificación coalescida (debounce corto).
2. Leer y calcular versión/hash real del archivo cuando se estabilice.
3. Comparar contra `base` y estado de edición de la sesión.
4. Si limpio y distinto → recargar, reconstruir nodos y actualizar vista.
5. Si sucio y distinto → `Conflict`; banner persistente.
6. Ignorar evento producido por nuestro propio guardado **solo si** el snapshot nuevo coincide con la versión confirmada; nunca por el mero nombre del proceso.
7. Revalidar siempre al guardar, incluso si el monitor no emitió evento.

No fusionar automáticamente cambios en v0.1. En `Revisar conflicto`, una comparación mínima puede mostrar qué claves existen/cambiaron, cuántas líneas varían y qué archivo está abierto, **sin mostrar valores secretos por defecto**. Esto es revisión, no editor de merge de tres vías.

## 8. Persistencia y política de guardado seguro

### 8.1 Interfaz del backend

```rust
pub trait DocumentStorage {
    fn read(&self, resource: &ResourceRef) -> Result<ReadSnapshot, StorageError>;
    fn capabilities(&self, resource: &ResourceRef) -> StorageCapabilities;
    fn replace_if_unchanged(
        &self,
        resource: &ResourceRef,
        expected: &DiskVersion,
        content: &[u8],
    ) -> Result<DiskVersion, StorageError>;
    fn create_new(&self, resource: &ResourceRef, content: &[u8])
        -> Result<DiskVersion, StorageError>;
    fn remove_stale_staging(&self, resource: &ResourceRef);
}

pub struct StorageCapabilities {
    pub supports_conditional_replace: bool,
    pub supports_atomic_replace: bool,
    pub can_preserve_permissions: bool,
    pub is_writable: bool,
}
```

> **Estado de implementación (2026-11-25):** el puerto real está en `src/services/storage.rs`. `create_new` (Guardar como, D-27) nunca sobrescribe ni sigue el destino; `remove_stale_staging` limpia al abrir los temporales huérfanos (D-12, revisión del 2026-11-22). El contrato detallado (precedencia de errores, `PostWriteVerificationFailed`) vive en los comentarios del puerto.

Interfaz conceptual; métodos síncronos mostrados por claridad. La lectura, hash y escritura deben ejecutarse sin bloquear el hilo principal de GTK (operaciones asíncronas Gio o worker controlado).

### 8.2 Guardado

**Algoritmo lógico:**

1. Verificar que el documento es UTF-8 editable y que los comandos actuales serializan con éxito.
2. Producir `bytes = serialize(document)` **sin** tocar la fuente todavía.
3. Verificar identidad/recurso (y política de symlink) y que las capacidades garantizan una escritura segura según el contexto.
4. Leer/consultar versión actual y compararla con `base`.
5. Ejecutar reemplazo condicionado al `etag` cuando el proveedor lo ofrezca, o mediante estrategia local que permita evitar sobrescrituras concurrentes de forma verificable.
6. Si cambia la versión durante la operación o el proveedor no puede garantizar lo necesario, devolver conflicto/error; **no** declarar éxito.
7. Tras éxito, volver a consultar/leer estado y metadatos; confirmar permisos, actualizar `base` y marcar limpio.
8. Si falla, conservar todo el documento y el historial en memoria, sin descartarlos.

**Importante:** verificar hash y luego renombrar por separado **no elimina por sí solo** la carrera entre ambas operaciones (TOCTOU). No documentar como garantía de concurrencia atómica una solución basada únicamente en `mtime` o en comprobación seguida de rename. Priorizar `Gio.File.replace_contents` con `etag` donde sea aplicable y probar su semántica real por backend; si la garantía no se cumple, bloquear una sobrescritura riesgosa y ofrecer `Guardar como…`.

Gio utiliza reemplazo atómico en ciertos backends locales, pero eso **no garantiza** que permisos, propietario, ACL, atributos extendidos, symlinks y comportamiento del portal coincidan entre todos los sistemas de archivos. Se requiere una matriz de pruebas: Linux local, permiso `0600`, `0644`, sin permiso de escritura, enlace simbólico, directorio sin permiso de creación y acceso por portal Flatpak.

**Política MVP de enlaces simbólicos:** detectar y rechazar por defecto la edición sobre la ruta simbólica. El usuario puede seleccionar expresamente el destino regular mediante el selector, sujeto a permisos. Se evita reemplazar accidentalmente el enlace por un archivo.

**Backups:** `make_backup=false` por defecto. No crear archivos `.bak` de secretos de manera implícita.

> **Estado de implementación (2026-11-25):** el spike ENV-T025 y la revisión de seguridad de ENV-T028 mostraron que `gio::File::replace` puede truncar el original cuando no puede crear su temporal. Por decisión del usuario (D-12, revisión del 2026-11-22), el guardado local no usa `replace_contents`: escribe un temporal privado `.envy-<hex>.tmp` en el mismo directorio anclado por descriptor, copia uid/gid/modo, hace `fsync`, comprueba el inodo base justo antes y publica con `renameat` (o `renameat2(RENAME_NOREPLACE)` en `create_new`), seguido de `fsync` del directorio y verificación posterior. Gio se mantiene para lectura, metadatos, monitor y URIs. La matriz de pruebas está en [`docs/testing/SAVE_SAFETY_MATRIX.md`](testing/SAVE_SAFETY_MATRIX.md).

## 9. Quick Edit y ciclo de vida de aplicación

- Binario nativo: `envy`, `envy .env`, `envy /ruta/absoluta/.env`.
- GTK/Gio: usar `Application` con `HANDLES_OPEN` y el evento `open` para archivos recibidos desde desktop y CLI; utilizar `HANDLES_COMMAND_LINE` únicamente si se necesitan opciones personalizadas que no encajan en apertura estándar.
- Una sola instancia primaria por app ID; abrir una ruta en instancia existente no debe eliminar ni reemplazar cambios de otras sesiones.
- Reutilizar componentes de `EditorView` en Quick Edit y Workspace; la diferencia es la navegación/contexto, no el motor.
- Si se abre un archivo ya activo, enfocarlo; no crear estados divergentes para la misma URI canónica cuando se puede determinar la identidad con seguridad.
- `envy --help` y asociación de archivos serán parte del empaquetado. Evitar reclamar la asociación MIME genérica de todos los archivos de texto.
- **Flatpak:** los argumentos y accesos desde terminal pueden pasar por wrappers o portales; validar un flujo oficial documentado (por ejemplo `flatpak run <app-id> ...` en los casos compatibles) sin presuponer que la ruta host es legible dentro del sandbox.

## 10. UX/UI GNOME nativa

### 10.1 Contenedores

```text
AdwApplicationWindow
└── AdwToastOverlay
    └── AdwToolbarView
        ├── AdwHeaderBar
        ├── AdwBanner (conflictos persistentes)
        └── RootStack / navigation state
            ├── QuickEditPage (editor sin sidebar)
            └── WorkspacePage
                └── AdwNavigationSplitView
                    ├── Sidebar: proyectos, servicios, archivos
                    └── Content: EditorView compartido
```

Son componentes de diseño; `RootStack` puede implementarse con `GtkStack` o con navegación libadwaita si simplifica el ciclo de vida de múltiples documentos.

### 10.2 Controles

| Control | Widget sugerido | Notas |
|---|---|---|
| Ventana | `AdwApplicationWindow` | Estilo del entorno GNOME |
| Header y acciones | `AdwHeaderBar`, `AdwToolbarView` | Atajos de aplicación mediante `GAction` |
| Navegación | `AdwNavigationSplitView` | Se colapsa en ventana estrecha |
| Lista de proyectos | `GtkListBox` + filas nativas | No reproducir una tabla web con tarjetas innecesarias |
| Valor corto | `AdwEntryRow` | Pegar, copiar, foco y accesibilidad estándar |
| Valor oculto | `AdwPasswordEntryRow` | Ofrecer revelar/ocultar por fila; no garantía de secreto en memoria |
| Valor multilínea | `GtkTextView` dentro de fila expandible | Permitir wrap y desplazamiento sin expandir toda la ventana |
| Búsqueda | `GtkSearchBar` / `GtkSearchEntry` | Buscar por clave sin filtrar secretos |
| Aviso de conflicto | `AdwBanner` + `AdwAlertDialog` | Persistir hasta resolver |
| Éxito recarga/guardado | `AdwToastOverlay` | Mensajes breves, no interrupciones |
| Selector | `GtkFileDialog` | Integración con portal de archivos |
| Ajustes | `AdwPreferencesDialog` según runtime objetivo | Configuración mínima |

`AdwNavigationSplitView` y `AdwToolbarView` se incorporaron en libadwaita 1.4; utilizar esa versión como mínimo preliminar, o subirla explícitamente si algún otro widget lo requiere. Los prototipos que representan la app como tarjetas son solo bocetos de jerarquía visual; la implementación respeta la GNOME HIG.

### 10.3 Estados de UI

- **Welcome:** sin proyectos; CTA `Abrir archivo`, `Agregar proyecto`.
- **Empty workspace:** proyecto sin archivos detectados; indicar dónde se buscó y permitir agregar carpeta de servicio o abrir otro archivo.
- **Editor clean:** título y contador de variables, Guardar desactivado.
- **Editor dirty:** indicador `Modificado` y Guardar disponible.
- **Conflict:** banner persistente, acciones de recuperación seguras; nunca autodescartar cambios.
- **Read only / Raw:** indicar explícitamente restricciones y origen.
- **Missing file / Access revoked:** preservar buffers locales y ofrecer reabrir/guardar como.
- **Paste preview:** mostrar claves, cantidad, conflictos y errores; no mostrar secretos en claro por defecto.

### 10.4 Colores por entorno

| Etiqueta | Convención visual orientativa | Detección |
|---|---|---|
| Development | Verde suave | `development`, `dev`, `local` |
| Staging | Violeta/azul suave | `staging`, `stage` |
| Production | Ámbar/rojo moderado | `production`, `prod` |
| Other | Neutro | Sin coincidencia |

Usar tokens del tema/libadwaita; no hardcodear colores que fallen contraste en light/dark o alto contraste. El usuario puede corregir la etiqueta. El color nunca implica protección especial del archivo ni sustituye texto visible.

## 11. Workspace, descubrimiento y configuración

### 11.1 Algoritmo de descubrimiento

Para cada proyecto:
1. Enumerar solo archivos inmediatos de su carpeta raíz.
2. Enumerar solo archivos inmediatos de cada carpeta de servicio asociada explícitamente.
3. Incluir nombres exactamente coincidentes con `.env`, `.env.*`, `*.env` (incluyen `.env.example`, `.env.local`, `service.env`).
4. Omitir carpetas, dispositivos, FIFOs y rutas simbólicas de edición automática; mostrar aviso si un archivo simbólico encaja en patrón.
5. Ordenar `.env` primero, luego otros por nombre natural; no leer valores para construir el listado.
6. Actualizar ante cambio de directorio mediante monitor cuando esté disponible o mediante comando `Refrescar`.

**Regla:** no recorrer `node_modules`, `.git`, submódulos ni directorios no registrados. El usuario controla qué carpetas integran el Workspace.

### 11.2 Configuración

Ejemplo indicativo (sin secretos):

```toml
schema_version = 1

[[projects]]
id = "project-medicore"
name = "Medicore"
uri = "file:///home/user/Projects/medicore"

[[projects.services]]
id = "service-backend"
name = "Backend"
uri = "file:///home/user/Projects/medicore/backend"

[[projects.environment_labels]]
file_uri = "file:///home/user/Projects/medicore/backend/.env.production"
kind = "production"
```

La ubicación recomendada para el catálogo es `$XDG_CONFIG_HOME/envy/projects.toml` en instalación nativa (o ruta propia del sandbox para Flatpak). GSettings puede contener preferencias como tamaño de ventana y modo de visualización; no almacenar últimas claves/valores editados. Escrita la configuración, validar esquema, evitar duplicados y migrar versiones de forma explícita.

## 12. Flatpak, portales y mínimos privilegios

La aplicación nativa y la aplicación Flatpak tienen capacidades de acceso distintas:

- `GtkFileDialog` usa mecanismos compatibles con portales para que el usuario elija archivos o carpetas.
- Elegir **un archivo** mediante portal no autoriza automáticamente a enumerar su carpeta ni a abrir archivos hermanos; Workspace necesita acceso explícito a la **carpeta** seleccionada.
- La persistencia de los permisos del portal y el comportamiento de rutas de documentos dependen de implementación y versiones; Envy debe probar reabrir proyectos tras reiniciar sesión/PC.
- Un archivo accesible mediante URI `document portal` puede no soportar exactamente las mismas operaciones que una ruta POSIX local; el backend de almacenamiento debe consultar capacidades.
- No solicitar `--filesystem=home` de manera silenciosa solo para simplificar el desarrollo. Si resulta necesario ampliar permisos para Workspace, justificarlo, documentarlo y validarlo antes de Flathub.

**Pruebas obligatorias:** agregar proyecto por selector dentro de Flatpak → enumerar dotenv → editar → guardar → salir → reiniciar Envy → volver a abrir el proyecto → editar de nuevo → observar modificación externa → detectar conflicto. También revisar si `Gio.File.replace_contents` puede escribir conservando metadatos para cada tipo de recurso que se prometa soportar.

## 13. Privacidad y amenazas

| Amenaza | Control |
|---|---|
| Valor secreto en log | No incluir contenido de archivos en diagnósticos, panics ni telemetría |
| Contraseña visible en UI | Ocultar heurísticamente y por elección del usuario; vista no equivale a cifrado |
| Secreto copiado al portapapeles | Copia solo por acción explícita; advertencia en documentación |
| Pérdida de modificaciones por watcher | State machine + nunca autodescartar si hay cambios locales |
| Sobrescritura en carrera | Backend de guardado condicionado + rechazo cuando falte garantía suficiente |
| Symlink o archivo especial | Detección y rechazo por defecto |
| Archivos inesperados en descubrimiento | Sin recursión implícita; carpetas explícitas |
| Pega contenido malicioso | Tratar como datos; nunca ejecutar shell ni expansiones |
| Archivo con sintaxis desconocida | Mantener como `Raw`, no normalizar |
| Copias persistentes en backups | No generar backups automáticamente |

## 14. Estrategia de pruebas

### 14.1 Fixtures de fidelidad (prioridad P0)

Cada fixture se carga, se parsea y se serializa sin editar. **La comparación es byte a byte**, no por string normalizada.

```text
fixtures/
  empty.env
  plain.env
  quoted-single.env
  quoted-double.env
  quoted-multiline.env
  comments-spacing.env
  inline-comments.env
  duplicate-keys.env
  mixed-newlines.env
  no-trailing-newline.env
  utf8-bom.env
  unicode-values.env
  unknown-syntax.env
  export-prefix.env
  invalid-utf8.bin
  unclosed-quotes.env
```

Pruebas mínimas:
- `serialize(parse(bytes)) == bytes` para todos los UTF-8 admitidos sin cambio.
- Modificar una variable mantiene intactos todos los spans no afectados.
- Lote de pegado + undo devuelve exactamente el estado original en memoria.
- Redo devuelve exactamente el estado del lote aplicado.
- Duplicados se conservan y una eliminación seleccionada no elimina el resto.
- Comillas simples, dobles y multilínea no pierden delimitadores ni fin de línea.
- Valores Unicode y BOM no invalidan rangos de bytes.
- Sintaxis `Raw` y errores parciales no desaparecen ni se “reparan”.

### 14.2 Pruebas de persistencia y conflictos

| Caso | Resultado esperado |
|---|---|
| Sin cambios locales + edición externa | Auto-reload y aviso discreto |
| Con cambios locales + edición externa | Estado conflicto; buffers locales intactos |
| Cambio externo justo antes de guardar | Operación rechazada por versión distinta |
| No evento de monitor, pero disco cambió | Guardado detecta conflicto en precondición |
| Archivo eliminado mientras se edita | Guardado bloqueado; usuario puede `Guardar como` |
| Archivo con modo `0600` | Privacidad de permisos conservada o error seguro |
| Archivo sin permisos de escritura | No se pierde estado en memoria |
| Symlink | No se reemplaza ni se sigue inadvertidamente |
| Directorio sin capacidad de archivo temporal | Error seguro, no truncado parcial silencioso |
| URI del portal Flatpak | Guardado y versionado probados; si no, modo no compatible y error explícito |
| Monitor emite eventos repetidos | No recargas múltiples que borren el estado |

### 14.3 UI y accesibilidad

- Teclado completo para abrir/buscar/editar/guardar/deshacer.
- Focus correcto entre sidebar, editor, diálogo de pegado y conflicto.
- Ventanas estrechas y anchas, light/dark y alto contraste.
- Filas de multilinea, duplicados, comentarios y `Raw` representadas sin ambigüedad.
- El texto `Production` sigue disponible sin depender exclusivamente del color.
- Copiar documento con cambios sin guardar devuelve exactamente la serialización en memoria.

### 14.4 CI

Mínimo `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`; construir UI en un runner Linux con dependencias GTK instaladas, y pruebas end-to-end del Flatpak en entorno gráfico/runner adecuado como hito de release. No ejecutar tests de GTK en un runner sin display y reportarlos como aprobados.

## 15. Secuencia recomendada de implementación

### Hito 1 — Núcleo puro Rust

1. Modelos `EnvDocument`, `EnvNode`, `AssignmentSyntax`.
2. Parser y serializador sin pérdida, con fixtures.
3. Operaciones mínimas `SetValue`, `Insert`, `Remove`, `Batch`.
4. Undo/redo, deduplicación diagnóstica y comparación de claves.
5. Política de guardado y pruebas de concurrencia antes de UI.

**Salida:** tests verdes y CLI de prueba interna (no es interfaz final) que demuestra fidelidad de archivos.

### Hito 2 — Quick Edit

1. Inicializar `gtk4`/`libadwaita` y recursos.
2. UI compartida `EditorView`: búsqueda, filas, multilinea, ocultación.
3. CLI nativa/apertura por selector, acciones de guardar y copiar.
4. Monitor Gio y avisos de conflicto.
5. Integrar pegado con preview y undo/redo.

**Salida:** `envy .env` resuelve el problema original sin proyecto registrado.

### Hito 3 — Workspace

1. Catálogo sin secretos y modelos `Project`/`Service`.
2. Registro explícito de carpetas; enumeración de patrones.
3. `AdwNavigationSplitView` y navegación adaptable.
4. Etiquetas de entorno y comparación con `.env.example`.
5. Indicadores de duplicados, estados vacíos y accesibilidad.

**Salida:** alternar archivos de múltiples proyectos sin navegar al filesystem manualmente cada vez.

### Hito 4 — Publicación

1. Auditar permisos de escritura y casos Flatpak; resolver bloqueadores.
2. Tests de fixtures y conflictos en sistema real.
3. README, traducción preparada, metainfo, icono, licencia y capturas.
4. Empaquetar v0.1.0, comprobar asociación y comando de apertura.

**Salida:** release reproducible con limitaciones expresamente documentadas.

## 16. Riesgos abiertos / decisiones a validar

| Riesgo/decisión | Validación requerida | Bloquea release |
|---|---|---|
| Concurrencia entre check y save | Confirmar semántica `etag`/conditional replace de backend elegido | Sí |
| Permisos POSIX y metadatos | Verificar `0600`, propietario y comportamiento de reemplazo | Sí |
| Flatpak acceso a carpetas y persistencia | Integración por portal, reinstancia y reinicio | Sí para distribuir Flatpak con Workspace |
| Compatibilidad de escapes entre dialectos | Definir `ValueCodec` y fixtures específicos | Sí para sintaxis anunciada |
| App ID, licencia y nombre | Confirmar GPL-3.0-or-later, elegir identificador definitivo y revisar disponibilidad | Sí para publicación |
| Asociación MIME y CLI Flatpak | Probar aperturas desde escritorio y terminal | Sí para función que se anuncie |
| Reordenamiento de filas | Fuera del MVP | No |
| Iconos por servicio | Fuera del MVP | No |

## 17. Referencias técnicas oficiales consultadas

- [Libadwaita — Adaptive Layouts](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/adaptive-layouts.html)
- [AdwNavigationSplitView — API](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.NavigationSplitView.html)
- [AdwPasswordEntryRow — API](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.PasswordEntryRow.html)
- [gtk-rs — GTK Application](https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/struct.Application.html)
- [Gio.File.replace_contents — API](https://docs.gtk.org/gio/method.File.replace_contents.html)
- [Gio.FileMonitor — API](https://docs.gtk.org/gio/class.FileMonitor.html)
- [XDG Desktop Portal — FileChooser](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.FileChooser.html)
- [Flatpak — Sandbox permissions](https://docs.flatpak.org/en/latest/sandbox-permissions.html)
- [GNOME Human Interface Guidelines](https://developer.gnome.org/hig/)

---

**Principio de salida:** Envy debe ser un editor que el usuario pueda dejar abierto mientras el IDE, una herramienta de compilación o un agente de IA toca los mismos archivos. Esa simultaneidad no debe convertirse en pérdida silenciosa de información.
