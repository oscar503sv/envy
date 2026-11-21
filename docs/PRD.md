# Envy — Product Requirements Document (PRD)

**Producto:** Envy  
**Versión del documento:** 1.1 — revisión consolidada  
**Release objetivo:** v0.1.0 (MVP)  
**Fecha:** 19 de noviembre de 2026  
**Estado:** Especificación de producto propuesta, lista para implementación por hitos  
**Plataforma inicial:** Linux / escritorio GNOME  
**Stack previsto:** Rust + GTK4 + libadwaita  
**Licencia propuesta:** GPL-3.0-or-later (pendiente de confirmar antes de publicar)

> **Visión:** *Your environments, beautifully organized.* Envy es un editor de archivos dotenv local-first y nativo de GNOME. Permite abrir un archivo y cambiar una variable en segundos (Quick Edit), o trabajar con los archivos de entorno de múltiples proyectos (Workspace), sin necesidad de abrir un IDE ni sacrificar la fidelidad del archivo original.

---

## 1. Problema y oportunidad

Los desarrolladores modifican archivos `.env` de forma frecuente para alternar endpoints, configurar bases de datos, activar opciones de desarrollo o revisar variables faltantes. Abrir un IDE para realizar una modificación de segundos introduce fricción. En proyectos con varios servicios y entornos, además, resulta difícil localizar el archivo correcto y comprobar si faltan claves.

Los editores de texto generales no ofrecen de forma integrada una vista estructurada de asignaciones, protección visual de secretos, navegación por proyectos ni aviso de conflictos cuando otro proceso —incluido un agente de IA— modifica el archivo.

Envy resuelve ese problema **sin convertirse en un gestor de secretos remoto, un IDE o una herramienta de despliegue**.

## 2. Objetivos y no objetivos

### 2.1 Objetivos del MVP

1. Abrir y modificar un archivo dotenv de manera directa desde terminal o selector gráfico.
2. Mantener una vista Workspace con proyectos locales registrados y sus archivos de entorno.
3. Preservar exactamente todos los bytes no modificados del archivo y evitar reformateos no solicitados.
4. Admitir asignaciones con valores sin comillas, con comillas simples o dobles, incluidos valores multilínea entre comillas.
5. Facilitar búsqueda, creación/eliminación de claves, pegado de asignaciones, copia y undo/redo.
6. Recargar cambios externos si la edición local está limpia y bloquear las sobrescrituras silenciosas si está sucia.
7. Distinguir visualmente entornos y comparar las claves de un archivo con `.env.example`.
8. Mantener secretos y datos de proyectos exclusivamente en el dispositivo, sin telemetría.

### 2.2 No objetivos

- Sin cuentas, backend, nube o sincronización entre equipos.
- Sin integración con SSH, Docker, Kubernetes, repositorios remotos, CI/CD o proveedores de secretos.
- Sin ejecución de comandos, inyección de variables a procesos o despliegue de aplicaciones.
- Sin interpretar/ejecutar shell, sustituciones de variables, expansiones ni código contenido en `.env`.
- Sin drag & drop de filas en v0.1; sí undo/redo de las ediciones admitidas.
- Sin iconos personalizados por servicio, histórico persistente de cambios ni comparación valor a valor entre entornos.
- Sin escaneo recursivo indiscriminado de directorios; descubrimiento limitado a carpetas registradas explícitamente.

## 3. Usuarios y situaciones de uso

**Usuario primario:** desarrollador individual de Linux que mantiene proyectos con archivos `.env` (backends, frontends, contenedores, monorepos).

**Situación A — Edición inmediata:** desde terminal dentro de un repositorio ejecuta `envy .env`, busca `DATABASE_URL`, actualiza su valor, guarda y vuelve al terminal.

**Situación B — Varios proyectos:** abre Envy desde el lanzador de GNOME, elige un proyecto registrado, selecciona su archivo de desarrollo y modifica variables.

**Situación C — Varios servicios:** dentro de un proyecto registra también `backend/` y `frontend/` como carpetas de servicio, para ver sus archivos sin buscar en el gestor de archivos.

**Situación D — Trabajo asistido por IA:** mantiene abierto `.env` mientras un agente actualiza el archivo. Envy detecta el cambio y no elimina ninguna de las modificaciones sin pedir una decisión.

**Situación E — Onboarding:** compara `.env` con `.env.example`, detecta una clave que falta y la añade sin copiar y pegar manualmente el archivo completo.

## 4. Modelo conceptual

| Término | Definición | Regla |
|---|---|---|
| **Proyecto** | Carpeta raíz registrada explícitamente por el usuario | No se copia a Envy; se almacena únicamente su referencia/ruta o URI |
| **Servicio** | Carpeta adicional, asociada a un proyecto, que contiene sus propios dotenv | Se agrega explícitamente; el nombre deriva de la carpeta pero se puede renombrar en Envy |
| **Entorno** | Etiqueta visual para clasificar un archivo (`Development`, `Staging`, `Production`, `Other`) | No altera ni determina automáticamente el contenido del archivo |
| **Archivo** | Recurso de sistema de archivos editable reconocido como dotenv | Puede abrirse directamente sin pertenecer a un proyecto |
| **Documento** | Representación editable en memoria de un archivo | Se conserva la versión base leída y las modificaciones no guardadas |
| **Asignación** | Ocurrencia individual de `KEY=VALUE` dentro de un documento | Las claves duplicadas son ocurrencias distintas, nunca se colapsan en un mapa |

Los valores de las variables **no** se guardan en la configuración de Envy. Workspace almacena identificadores, URI/rutas, nombres visuales, etiquetas y preferencias; los valores permanecen en los archivos originales.

## 5. Alcance funcional del MVP (prioridades)

**P0 = condición de seguridad o corrección sin la cual no se lanza. P1 = experiencia esencial. P2 = organización del Workspace.** Las tres prioridades forman parte del MVP; los hitos solo establecen el orden de construcción.

| ID | Prioridad | Función | Comportamiento del MVP |
|---|---|---|---|
| ENV-001 | P0 | Parser sin pérdida | Al abrir y guardar sin ediciones se obtiene un archivo idéntico a nivel de bytes |
| ENV-002 | P0 | Edición de asignaciones | Soporta valores sin comillas, comillas simples/dobles y multilínea entre comillas |
| ENV-003 | P0 | Sintaxis desconocida | Preservada y visible como contenido de solo lectura, sin interpretación inventada |
| ENV-004 | P0 | Guardado protegido | Detecta conflictos, conserva permisos relevantes y no informa éxito sin confirmación de escritura |
| ENV-005 | P0 | Monitoreo de archivo | Recarga automática si no hay cambios locales; muestra conflicto si los hay |
| ENV-006 | P1 | Quick Edit | `envy <archivo>` y apertura desde diálogo; no exige crear proyecto |
| ENV-007 | P1 | Buscar | Filtra por nombre de clave, sin mostrar ni indexar secretos fuera de memoria |
| ENV-008 | P1 | Crear/eliminar | Añade asignaciones, elimina solo la ocurrencia seleccionada y admite deshacer |
| ENV-009 | P1 | Undo/redo | Historial en memoria por documento para acciones admitidas |
| ENV-010 | P1 | Paste assignments | Pega una o más asignaciones, con previsualización y confirmación cuando hay duplicados o líneas no soportadas |
| ENV-011 | P1 | Copiar | Copia el valor seleccionado o la representación completa en memoria, incluidos cambios sin guardar |
| ENV-012 | P1 | Secretos | Ocultación visual por defecto según heurística y opción manual de revelar/ocultar |
| ENV-013 | P1 | Estado de cambios | Señal clara de `Sin cambios`, `Modificado`, `Conflicto` y `Solo lectura` |
| ENV-014 | P2 | Workspace | Agregar, listar, renombrar visualmente y quitar proyectos sin borrar carpetas |
| ENV-015 | P2 | Servicios | Asociar carpetas de servicio explícitamente a un proyecto |
| ENV-016 | P2 | Descubrimiento | Encuentra `.env`, `.env.*` y `*.env` en la raíz de cada carpeta registrada |
| ENV-017 | P2 | Etiquetas de entorno | Color más texto visible; asignación inicial por nombre, corregible manualmente |
| ENV-018 | P2 | Comparación de claves | Diferencias entre archivo elegido y `.env.example` de su misma carpeta |
| ENV-019 | P2 | Duplicados | Identifica claves repetidas sin asumir la que prevalece |
| ENV-020 | P1 | Atajos y accesibilidad | Acciones con teclado, foco visible, etiquetas accesibles y adaptación a ventana estrecha |

## 6. Reglas y criterios de aceptación

### US-01 — Abrir y editar directamente (ENV-006, ENV-007)

**Como** desarrollador, **quiero** ejecutar `envy .env` y encontrar el archivo listo para editar **para** no abrir un IDE.

**Criterios:**
- `envy .env` interpreta la ruta relativa desde el directorio actual (en instalación nativa); `envy /ruta/al/archivo` admite ruta absoluta. La CLI y la experiencia dentro del sandbox Flatpak se documentan por separado.
- Abrir sin argumento muestra la última vista disponible de Workspace o un estado de bienvenida si no hay proyectos.
- Abrir un archivo directo no lo registra automáticamente como proyecto.
- Si otro archivo ya está abierto con cambios sin guardar, Envy no los descarta; abre el nuevo documento en una sesión/vista independiente o pide una decisión apropiada.
- Archivo inexistente, inaccesible, no regular o no editable genera error legible sin crear archivos implícitamente.

### US-02 — Edición no destructiva (ENV-001, ENV-002, ENV-003)

**Como** desarrollador, **quiero** modificar `DEBUG=true` sin que la aplicación cambie comentarios, comillas o espaciado en otras líneas.

**Criterios:**
- Abrir y serializar sin cambios produce exactamente el mismo flujo de bytes, incluidos BOM UTF-8 opcional y terminaciones `LF`, `CRLF`, `CR` o mezcladas.
- Las líneas no modificadas se devuelven literalmente desde la fuente original.
- Se reconoce una asignación estándar con nombre `^[A-Za-z_][A-Za-z0-9_]*$`, espacio opcional alrededor de `=`, valores sin comillas o entre comillas simples/dobles y asignaciones `export KEY=...` donde sean parseables.
- Se admiten valores entre comillas que ocupen varias líneas físicas, con un control de edición de varias líneas.
- Los comentarios y sintaxis desconocida se visualizan pero no se editan con controles estructurados. No se ejecutan expresiones.
- Si una asignación tiene construcción no soportada, se presenta como `Raw` no editable y se mantiene íntegra. Si un archivo no es UTF-8 válido, se rechaza la edición estructurada indicando la razón; nunca se reescribe silenciosamente.
- Al modificar una asignación reconocida, Envy conserva todos sus fragmentos léxicos compatibles (prefijo, nombre, separador, estilo de comillas, sufijo). Si un nuevo valor no puede representarse de forma segura en el estilo actual, solicita conversión explícita o se niega a guardar esa edición.

### US-03 — Crear, eliminar, pegar, deshacer y rehacer (ENV-008, ENV-009, ENV-010)

**Criterios:**
- Nueva clave: se valida el nombre y se crea una asignación al final del documento, manteniendo el estilo de salto de línea dominante y respetando si el archivo acababa con newline.
- Eliminar: se elimina solo la ocurrencia elegida; una clave duplicada no causa la eliminación de sus otras apariciones.
- Pegar asignaciones: acepta uno o varios bloques `KEY=VALUE`, incluidas asignaciones multilínea parseables; presenta previsualización, resumen de inserciones y advertencias antes de aplicar el lote.
- Un bloque pegado con clave ya existente no reemplaza automáticamente la variable; el usuario escoge **añadir**, **reemplazar ocurrencia elegida** o **cancelar**. Si existen múltiples ocurrencias, no se infiere cuál reemplazar.
- Si parte del pegado no se puede interpretar, no se aplica parcialmente: se informa y se permite cancelar o insertar únicamente las entradas válidas tras una confirmación explícita.
- Undo/redo funciona para editar valor, crear, eliminar y pegar en lote. La acción de pegado se deshace en una sola operación. El historial es por documento, en memoria y no persiste después de cerrar.
- Después de guardar, las acciones previas del historial pueden mantenerse en la sesión, pero un undo posterior vuelve a marcar el documento como modificado; nunca revierte automáticamente el archivo en disco sin una nueva orden de guardar.

### US-04 — Copiar correctamente (ENV-011, ENV-012)

**Criterios:**
- Copiar valor copia los bytes/texto del valor editable actual, no el valor antiguo del disco.
- Copiar documento copia la serialización actual, aunque existan cambios sin guardar, incluyendo comentarios y líneas `Raw`.
- Ninguna operación de copia ocurre automáticamente. La interfaz indica que el portapapeles puede conservar secretos fuera del control de Envy; no promete borrado fiable del portapapeles.
- La máscara de secretos solo es una protección visual. El contenido sigue estando presente en memoria y en el archivo original.

### US-05 — Detectar cambios externos (ENV-004, ENV-005, ENV-013)

**Criterios:**
- Si cambia el contenido en disco y el documento no está modificado localmente, se vuelve a cargar, se actualiza la vista y se muestra un mensaje discreto.
- Si cambia el contenido en disco y hay ediciones sin guardar, no se recarga ni guarda automáticamente. Se muestra un aviso persistente con `Recargar y descartar mis cambios`, `Guardar como…` y `Revisar conflicto` (este último expone diferencias o un resumen básico de versiones en el MVP).
- La opción de sobrescribir el disco, si se ofrece, requiere confirmación clara y una nueva comprobación antes de la escritura; ante un nuevo conflicto, se bloquea.
- La detección no depende solo de `mtime`: al guardar se verifica la versión/base por hash y, cuando esté disponible, por `etag` del proveedor; los eventos de monitorización solo disparan la comprobación.
- Si el archivo desaparece, se mueve o cambia a solo lectura, Envy lo informa y conserva los cambios locales en memoria hasta que el usuario decida.

### US-06 — Workspace y servicios (ENV-014, ENV-015, ENV-016)

**Criterios:**
- Registrar una carpeta crea una entrada de proyecto. Agregar una carpeta de servicio es opcional y explícito.
- Se buscan archivos únicamente en la raíz de la carpeta del proyecto y de sus carpetas de servicio registradas; no se atraviesan dependencias ni repositorios anidados.
- Los nombres de archivo admitidos son exactamente `.env`, `.env.*`, `*.env`. No se confunden por defecto `.envrc`, `.env.example.txt` ni otros formatos.
- Un proyecto puede no tener ningún `.env` y se muestra un estado vacío útil.
- Quitar un proyecto de Envy elimina únicamente la referencia local; **jamás** elimina su carpeta ni sus archivos.
- Los proyectos registrados se restauran tras reiniciar la aplicación, siempre que los permisos del entorno (especialmente Flatpak) lo permitan.

### US-07 — Entornos, plantillas y duplicados (ENV-017, ENV-018, ENV-019)

**Criterios:**
- Las etiquetas iniciales se infieren por nombres conocidos (`production`/`prod`, `staging`/`stage`, `development`/`dev`, `local`) y son editables; los ambiguos quedan como `Other`.
- El color siempre va acompañado por texto/iconografía; no debe ser la única señal.
- Al comparar un archivo con un `.env.example` de la misma carpeta se identifican claves `Faltan`, `Solo en archivo actual` y `En ambos`. No se comparan valores ni se exponen secretos.
- Un clic en una clave faltante puede preparar su inserción, con valor inicialmente vacío y confirmación del usuario; no hereda secretos de otro archivo.
- Los duplicados se advierten con número de ocurrencias; su interpretación queda al runtime del usuario, no a Envy.

## 7. Experiencia y navegación

### 7.1 Entrada a la aplicación

- **Sin argumento:** Workspace (si tiene proyectos), o bienvenida con acciones `Abrir archivo` y `Agregar proyecto`.
- **Con ruta:** Quick Edit contextual, sin navegación de proyectos obligatoria.
- **Archivo del gestor:** apertura mediante mecanismo de aplicación GNOME cuando MIME/asociación y permisos sean compatibles.

### 7.2 Workspace

En pantalla ancha, panel lateral con proyectos y sus servicios/archivos; zona principal con el editor. En ventana estrecha, navegación colapsable con flujo de ida y vuelta. En la barra superior: nombre de archivo, buscar, agregar variable, abrir archivo y menú de aplicación. El estado `Modificado` debe ser visible en la fila del archivo y en el editor.

### 7.3 Editor

- Cada asignación reconocida tiene clave, control de valor y acciones contextuales para copiar, mostrar/ocultar y eliminar.
- Valor de una línea: entrada compacta; valores multilínea: control expandible.
- Comentarios y contenido no reconocido se muestran con una etiqueta `Sin modificar` / `Sintaxis no editable`; deben ser seleccionables cuando sea viable, sin convertirse en un editor de código general.
- Campo de búsqueda visible bajo demanda; filtra ocurrencias por clave y muestra un contador.
- El control de Guardar está deshabilitado cuando no hay cambios o el recurso es de solo lectura. `Ctrl+S` siempre tiene comportamiento consistente.
- Mensajes transitorios: toast (`Guardado`, `Recargado`); conflictos que requieren decisión: banner persistente y/o diálogo.

### 7.4 Atajos

| Atajo | Acción |
|---|---|
| `Ctrl+O` | Abrir archivo |
| `Ctrl+S` | Guardar archivo |
| `Ctrl+F` | Buscar variables |
| `Ctrl+N` | Agregar asignación |
| `Ctrl+Z` / `Ctrl+Shift+Z` | Deshacer / rehacer |
| `Ctrl+C` | Copiar selección con foco; acción explícita para copiar valor o documento completo |
| `Esc` | Cerrar búsqueda, menú o diálogo cuando sea apropiado |

Usar aceleradores GNOME convencionales; verificar accesibilidad y conflictos con el foco de los campos de texto.

## 8. Reglas de seguridad y privacidad

1. Sin red ni telemetría; Envy no envía valores a ningún servicio.
2. Sin registro de contenidos de valores en `tracing`, panics, mensajes de error, analíticas ni preferencias.
3. La detección de secretos (`SECRET`, `TOKEN`, `PASSWORD`, `KEY`, etc.) es heurística y configurable por fila; **no** garantiza ocultar todo material sensible.
4. No generar respaldos automáticos con secretos. Un futuro respaldo será opt-in y explícito.
5. No editar enlaces simbólicos por defecto en el MVP; informar al usuario y permitir abrir explícitamente la ruta de destino mediante una nueva selección autorizada, si existe.
6. No tratar contenido dotenv como shell ejecutable. No ejecutar expansiones, comandos ni plantillas.
7. Verificar identidad/versionado del recurso en apertura y antes de guardar; no sobrescribir silenciosamente.
8. Fallar de manera visible y segura si no se pueden garantizar los permisos mínimos o el modo de guardado del proveedor de archivos.
9. Los permisos de Flatpak deben ser limitados; acceso adicional únicamente mediante selección del usuario o consentimiento explícito.

## 9. Restricciones técnicas y compatibilidad

- Ubuntu/Fedora/Arch y otros escritorios Linux con GTK4/libadwaita compatibles son objetivos generales; el primer entorno de pruebas oficial se definirá para el release.
- UTF-8 es el formato de edición estructurada soportado en v0.1; otros encodings no se reescriben.
- La sintaxis dotenv no es universal: la compatibilidad documenta un subconjunto explícito y preserva cualquier entrada no admitida sin reinterpretarla.
- Flatpak necesita una prueba específica de permisos persistentes sobre carpetas, detección de archivos, CLI y guardado seguro: **no se presupone** que el portal de selección de un archivo conceda acceso a sus archivos hermanos.
- El MVP soporta archivos locales. Recursos remotos o GVfs no se prometen sin validar escritura y monitoreo.

## 10. Métricas del producto y calidad

- **Métrica principal:** tiempo para abrir, localizar y guardar una variable en un archivo ya conocido.
- **Calidad no negociable:** 100 % de fixtures sin cambios se serializan byte a byte de manera idéntica.
- **Cero pérdidas conocidas:** ningún test permite descartar ediciones locales o externas sin una acción expresa.
- **Interacción:** completar Quick Edit sin necesidad de registrar proyecto.
- **Privacidad:** los logs de pruebas no contienen valores secretos de fixtures.
- **Accesibilidad:** todas las acciones esenciales se pueden completar con teclado; los estados y colores tienen texto alternativo.

Los objetivos numéricos de tiempo/rendimiento se fijarán después de crear una línea base con hardware real; se evita prometer milisegundos sin medición.

## 11. Hitos de entrega

1. **Motor de documentos (P0):** parser, serializador, Unicode, multilínea, tests de byte fidelity, política de escritura y conflictos.
2. **Quick Edit (P1):** CLI nativa, UI básica, edición, crear/eliminar, buscar, copiar, pegado, undo/redo.
3. **Workspace (P2):** proyectos, carpetas de servicio, descubrimiento, etiquetas de entorno, plantillas y duplicados.
4. **Endurecimiento/release:** pruebas Flatpak, navegación adaptable, accesibilidad, traducciones preparadas, errores y empaquetado v0.1.0.

**Criterio de salida:** todos los requisitos P0, P1 y P2 de esta versión completados o renegociados explícitamente en el PRD; ninguna regresión de fidelidad de archivo, secretos o conflictos.

## 12. Riesgos y mitigaciones

| Riesgo | Mitigación / decisión |
|---|---|
| Diferentes dialectos dotenv | Subconjunto documentado, nodos `Raw`, preservación literal y tests de fixtures |
| Multilínea, comillas y escapes | Parser con spans y tests de casos límite; nunca reconstruir todo a partir de un `HashMap` |
| Conflicto durante guardado | Comprobación optimista + soporte de `etag` donde exista; abortar si difiere |
| Guardado atómico/permisos en Flatpak | Backend por capacidades, prueba de integración con portal; no prometer operaciones no soportadas |
| Secretos expuestos en clipboard | Acciones explícitas y advertencia documental; nunca copiar automáticamente |
| Complejidad creciente | Prohibido añadir integraciones, drag & drop y automatizaciones antes de v0.1 |
| UI con demasiados campos | Controles compactos, búsqueda y filas expandibles; diseño adaptable nativo |

## 13. Decisiones pendientes antes de cerrar v0.1

- ID de aplicación (`io.github.<owner>.Envy` es provisional), identidad de proyecto, confirmación de GPL-3.0-or-later y verificación del nombre en tiendas/repositorios.
- Runtime GNOME/SDK mínimo y distribuciones de pruebas.
- Política exacta de acceso a carpetas en Flatpak: permisos por selección, portal y persistencia, o alternativa explícita con acceso ampliado —a validar técnicamente antes de publicar.
- Alcance real del soporte de `export`, escapes, comentarios al final de línea y otros dialectos, documentado como fixtures.
- Política definitiva cuando el backend no permite reemplazo atómico, conservación verificable de permisos o comparación de `etag`.

Estas son decisiones de implementación pendientes, **no** permisos para ampliar el alcance de producto.

## 14. Definición de terminado (DoD)

- [ ] Parser y serializador cumplen fidelidad byte a byte para todos los fixtures sin cambios.
- [ ] Edición de valores simples y multilínea cubierta con tests.
- [ ] Operaciones de crear/eliminar/pegar/copy/undo/redo cubiertas con tests.
- [ ] Guardado nunca sobrescribe silenciosamente modificaciones externas.
- [ ] Quick Edit funciona fuera de Workspace en instalación nativa.
- [ ] Proyectos, servicios, archivo de entorno, comparación de claves y duplicados funcionan.
- [ ] Permisos y flujo Flatpak validados en entorno real o limitaciones comunicadas y resueltas antes del release.
- [ ] UI GNOME adaptable, con teclado y sin contenido sensible en logs.
- [ ] README, captura de pantalla, licencia y metadatos de aplicación preparados.

---

**Documento complementario:** [`TECHNICAL_DESIGN.md`](./TECHNICAL_DESIGN.md). En caso de discrepancia sobre el alcance, prevalece este PRD; para decisiones internas de implementación, prevalece el diseño técnico salvo que contradiga un criterio de aceptación.
