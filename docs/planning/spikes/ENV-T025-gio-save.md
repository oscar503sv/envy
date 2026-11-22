# ENV-T025 — Spike: semántica real de guardado condicionado con Gio

## 0. Entorno

| Dato | Valor |
|---|---|
| GLib/GIO | 2.88.4 (pkg-config), Fedora 44, kernel 7.3.4, SELinux **Enforcing**, usuario no root (uid 1000, grupos 1000/10/962) |
| Crate `gio` | 0.22.11 (la familia 0.22 que usa gtk4 0.11.5, MSRV 1.92). Se resolvió `gio@0.22` sin `cargo tree` de gtk4 completo: gtk4 0.11.5 expone sus features `gio/v2_xx` sobre ese mismo crate |
| Sistemas de ficheros | **btrfs** (`/home`, directorio `~/.cache/t025-spike`, el relevante) y **tmpfs** (`/tmp`, scratchpad). Los 45 casos se ejecutaron en ambos; el comportamiento es idéntico salvo la etiqueta SELinux por defecto (cache_home_t vs user_tmp_t) |
| Código | Programa desechable fuera del repositorio (un `fn case_*` por caso); no se versiona |
| Resultados crudos | Salidas del programa en btrfs y tmpfs (no versionadas); los extractos relevantes están en la matriz |

Además de las pruebas empíricas se leyó el fuente de GLib 2.88.4 (`gio/glocalfileoutputstream.c`, descargado de gitlab.gnome.org) para explicar los resultados; las citas de líneas son de ese archivo.

## 1. Cómo funciona realmente `replace_contents` en local (fuente + observación)

`g_file_replace_contents` = `g_file_replace` (stream) + escritura + `close`. En `handle_overwrite_open`:

1. `open(O_WRONLY|O_CREAT|O_NOFOLLOW)` del archivo original. Falla con `PermissionDenied` si el archivo no es escribible por el usuario (0444).
2. `fstat` del original; si hay `etag`, se compara con el etag calculado de ese `stat` **en ese momento** (WRONG_ETAG si difiere). Este es el **único** punto de comprobación.
3. Si `nlink == 1`, no es symlink (o `REPLACE_DESTINATION`): crea `.goutputstream-XXXXXX` en el mismo directorio, `fchown`+`fchmod` al uid/gid/modo del original (salvo `REPLACE_DESTINATION`). Si crear el temporal falla, o si `fchown`/`fchmod` fallan **y** el temporal no quedó con los mismos uid/gid/modo, se pasa a `fallback_strategy`.
4. `fallback_strategy`: **`ftruncate(fd, 0)` sobre el propio archivo y escritura en el sitio** (no atómica). Se usa para: directorio padre sin permiso de escritura, enlace duro, enlace simbólico, fallo de chown/chmod. No hay ningún error ni señal en la API que indique que se usó.
5. En `close`: `fsync` del temporal (solo si el destino ya existía) y `rename(tmp, original)`. No se hace `fsync` del directorio. Con `make_backup=true` primero se hace `link`/`rename` original → `nombre~`.

Consecuencia: **el etag se comprueba al abrir el stream, no al hacer el rename**. La ventana TOCTOU abarca todo el tiempo entre la apertura del stream y el `close` (en `replace_contents`, la escritura del buffer + fsync + rename).

## 2. Matriz de resultados (btrfs; tmpfs idéntico salvo indicación)

Notación: inodo y modo "antes/después". Extractos de evidencia en la columna final.

| Caso | Setup | Resultado observado | Inodo | Modo | Evidencia clave |
|---|---|---|---|---|---|
| 1a formato etag | archivo nuevo | `"<mtime_s>:<mtime_µs>:<mtime_ns>"`, p. ej. `1795330872:459333:459333295`; deriva solo de mtime (no de tamaño/inodo/ctime) | – | – | coincide con `std` mtime y `time::modified[-usec/-nsec]` |
| 1b etag devuelto | replace sin etag | el etag devuelto por `replace_contents` **es igual** al consultado después con `query_info("etag::value")` | – | – | `iguales=true` |
| 1c reescritura mismo tamaño/segundo | 200 reescrituras de 4 B | 0/200 etags repetidos (resolución ns, no solo segundos) | – | – | |
| 1d chmod | 600→644 | etag **no** cambia | – | – | mismo valor antes/después |
| 1e touch | mtime +5 s | etag cambia | – | – | |
| 1f contenido distinto + mtime restaurado exacto | write + `set_modified(t0)` | etag **igual** (punto ciego: `rsync -t`, `cp -p`, `touch -r`, restauración de backups) | – | – | `...:0:0 vs ...:0:0` |
| 1g mtime +500 ns | | etag **distinto** (el ns cuenta en 2.88; en versiones antiguas del formato `sec:usec` no contaba) | – | – | `...:0:0` vs `...:0:500` |
| 2a conditional: contenido externo distinto | etag viejo | **WRONG_ETAG** (contenido externo conservado) | 2547077 / igual | 0600/0600 | |
| 2b mismo contenido reescrito | etag viejo | **WRONG_ETAG** (falso positivo benigno) | igual | 0600/0600 | |
| 2c solo touch | etag viejo | **WRONG_ETAG** | igual | 0600/0600 | |
| 2d solo chmod externo | etag viejo | OK, sobrescribe; **el modo externo 0640 se pierde sin que Gio lo note** salvo por la lógica de modo (ver 3) | 2547083 / 2547084 | 0600/0640 → se copia el *actual* (0640) | etag no cubre permisos |
| 2e contenido distinto + mtime restaurado | etag viejo | **OK: pisa el cambio externo sin aviso** | cambia | 0600/0600 | contenido final `MINE` |
| 2f TOCTOU (rename) | `replace(Some(etag))` → write externo → `write_all` → `close` | `close` OK; **el write externo se pierde sin error** | 2547089 / 2547090 | – | final `MINE` |
| 2g TOCTOU (en el sitio, nlink=2) | igual | `close` OK; contenido **corrupto/mezclado** `MINERNAL-WRITE-AFTER-CHECK` | igual | – | truncado en open + write externo + write propio |
| 3a–3d modos 0600/0644/0640/0755, NONE | | **modo conservado** en todos (vía fchmod del temporal) | cambia | idéntico | |
| 3e–3f PRIVATE sobre existente 0600/0644 | | modo conservado (PRIVATE no afecta existentes) | cambia | idéntico | |
| 3g **REPLACE_DESTINATION** 0600 | | **0600 → 0644: filtración de modo** (umask 022) | cambia | 0600/0644 | |
| 3h REPLACE_DESTINATION\|PRIVATE 0640 | | 0640 → 0600 (modo cambia igualmente) | cambia | 0640/0600 | |
| 3i/3j archivo inexistente NONE/PRIVATE | | 0644 (umask) / **0600** | – | –/0644, –/0600 | PRIVATE es la forma de crear 0600 |
| 3k grupo suplementario (docker 962), NONE | chown :962 | gid conservado (962), uid conservado | cambia | 0640/0640 | |
| 3l grupo 962, REPLACE_DESTINATION | | **gid → 1000 (se pierde)** | cambia | 0640/0644 | |
| 4a archivo 0444 en dir escribible | | **`PermissionDenied`**; archivo intacto, sin temporales | igual | 0444 | no se elude el bit de solo lectura |
| 4b 0444 + REPLACE_DESTINATION | | `PermissionDenied`, intacto | igual | 0444 | |
| 5a dir 0555, archivo 0644 | | **OK pero mismo inodo ⇒ escritura NO atómica en el sitio** | 2547133 / 2547133 | 0644 | `parent access::can-write=false` |
| 5b dir 0555 + etag correcto | | OK, mismo inodo (en el sitio); el etag sí se comprueba | igual | 0644 | |
| 5c dir 0555 + REPLACE_DESTINATION | | `PermissionDenied`, archivo intacto | igual | 0644 | |
| 5d dir 0555: disco tras abrir stream | `replace(None)` sin escribir | **archivo ya truncado a 0 bytes** (contenido `""`) | – | – | riesgo de pérdida de datos ante fallo/crash a mitad |
| 6a symlink, NONE | `link -> target` | **el enlace se conserva; se escribe el *target* en el sitio** (inodo del target igual, no atómico) | target igual | 0600 | link sigue siendo symlink |
| 6b etag vía symlink | etag leído por el link, target cambiado | **WRONG_ETAG** (el etag es del target) | – | – | |
| 6c symlink + REPLACE_DESTINATION | | **el enlace se sustituye por archivo regular 0644**; target intacto | – | 0600/0644 | |
| 6d symlink colgante, NONE | `-> missing.env` | **OK: crea `missing.env`** (target) 0644 | – | –/0644 | |
| 6e colgante + REPLACE_DESTINATION | | el enlace se sustituye por archivo regular | – | – | |
| 7a enlace duro, NONE | a y b nlink=2 | **enlace conservado** (b ve `NEW`), pero **en el sitio, no atómico** | 2547157 / igual | 0600 | b="NEW" |
| 7b enlace duro, REPLACE_DESTINATION | | **enlace roto**: b conserva `OLD`, nlink 2→1, modo 0600→0644 | cambia | 0600/0644 | |
| 8a make_backup=false | | sin `~`, sin temporales residuales | cambia | 0600 | dir=`a.env` |
| 8b make_backup=true | | crea `a.env~` (0600, contenido viejo con secretos) | cambia | 0600 | dir=`a.env,a.env~` |
| 9a xattr `user.*` + etiqueta SELinux, ruta rename | `setfattr`, `chcon -t ssh_home_t` | **ambos se pierden**: xattr desaparece; `ssh_home_t` → `cache_home_t` (etiqueta por defecto del directorio) | cambia | – | `getfattr -d -m -` antes/después |
| 9b igual, ruta en el sitio (nlink=2) | | ambos **se conservan** (mismo inodo) | igual | – | |

(En tmpfs: mismos resultados; 9a termina en `user_tmp_t`.)

Flags existentes (`gio-sys`): `NONE=0`, `PRIVATE=1`, `REPLACE_DESTINATION=2`. No hay flag de fsync (GLib hace `fsync` del temporal sólo si el destino existía, no del directorio).

## 3. Ventana TOCTOU residual (declaración explícita)

- Gio **no** ofrece reemplazo condicional atómico. El etag se compara una sola vez, al abrir el stream (paso 2 de §1); después viene escritura, `fsync` y `rename`. Un cambio externo posterior a esa comprobación **se pierde sin error** (2f) o mezcla contenido (2g, ruta en el sitio).
- Tamaño de la ventana: de la apertura del stream a `close`: escritura + `fsync` (ms en archivos .env, pero con disco lento, btrfs/fsync o red puede ser mucho mayor). No es cero ni acotable por la aplicación.
- El etag es solo mtime (ns): ciego a restauración de mtime (1f/2e) y no cubre permisos (2d). Un hash del contenido leído justo antes cubre 1f/2e, pero deja otra ventana propia (entre el hash y el open).
- Mitigación disponible sin garantía atómica: hash + etag inmediatamente antes, `replace` con etag, **verificación posterior** (releer; si el contenido no es exactamente el escrito, informar). La verificación posterior detecta que otro proceso escribió *después* de nuestro rename, **no** que nuestro rename pisó un cambio intermedio (en 2f la relectura devuelve `MINE`, indistinguible de éxito limpio).
- Por tanto la limitación debe documentarse como **"detección de cambios best-effort, no exclusión mutua"** (D-12.1).

**Cuándo hay que bloquear el guardado normal** (ver §4): ruta en el sitio previsible (dir padre no escribible, nlink>1, symlink, chown/chmod imposible), modo/propietario/grupo no conservable, WRONG_ETAG o hash distinto, archivo no escribible.

## 4. Algoritmo recomendado para ENV-T028

Veredicto: **hay garantías aceptables si se acepta D-12.1 (limitación TOCTOU documentada) y se antepone una comprobación previa que bloquea las rutas no atómicas.** No se necesita escalar; hay que escalar solo el punto de hard links (D-34) y la política de xattr/SELinux (nueva, ver 4.9). Gio no permite detectar *a posteriori* el uso del fallback de forma segura (el archivo ya se truncó), por lo que **la detección debe ser previa**.

Pasos (`replace_if_unchanged(path, bytes, base: {hash, etag, mode, uid, gid, ino})`):

1. **Resolución de recurso y política de symlink (D-12 / TD §8.2 / ENV-T043)**: `lstat`. Si es symlink: rechazar por defecto (Gio lo seguiría y escribiría el target en el sitio, no atómico: 6a; un colgante *crea* el target: 6d). Si el usuario elige el destino regular, operar sobre la ruta resuelta.
2. **Pre-vuelo "¿ruta atómica?" (D-12.3)**, todo sobre la ruta regular:
   - archivo regular, `nlink == 1` (si no: ver D-34 más abajo);
   - directorio padre escribible y ejecutable (`access::can-write` del padre vía `query_info`, o `faccessat(AT_EACCESS)`): evita 5a/5b/5d;
   - archivo escribible por el usuario (el open de Gio lo exigirá, 4a; el error `PermissionDenied` se muestra como solo lectura, **no** se intenta eludir);
   - uid propio == uid del archivo, o gid del archivo ∈ grupos del proceso (cuando el `fchown` del temporal puede fallar, Gio cae al modo en el sitio);
   - si cualquiera falla ⇒ **bloquear Guardar** y ofrecer **Guardar como…** (D-12.3). Opcionalmente, para el caso dir-sin-escritura, informar explícitamente "no se puede guardar de forma segura aquí".
3. **Hash + etag inmediatamente antes**: releer bytes, comparar hash con `base.hash` y `etag::value` con `base.etag`; si difieren ⇒ conflicto (D-14/ENV-T042).
4. **`replace_contents(bytes, Some(etag_base), make_backup=false, FileCreateFlags::NONE)`**:
   - **No usar `REPLACE_DESTINATION`** (pierde modo 3g, grupo 3l, rompe hard links 7b y reemplaza symlinks 6c).
   - **No usar `PRIVATE` sobre archivos existentes** (inocuo, pero innecesario); sí usar `PRIVATE` solo al crear archivos nuevos que deban ser 0600 (3j, p. ej. Guardar como… de un `.env`).
   - `WRONG_ETAG` ⇒ conflicto; cualquier otro error ⇒ conservar documento e historial (TD §8.2.8), sin éxito.
5. **Verificación posterior (TD §8.2.7)**: `lstat` + releer. Exigir: contenido == bytes escritos; **inodo distinto del base** (si es igual con `nlink==1` el fallback en el sitio ocurrió pese al pre-vuelo ⇒ registrar el incidente y avisar de que la escritura no fue atómica); modo, uid, gid iguales a los de `base`.
6. **Modo (D-12.2)**: si el modo difiere, intentar `fchmod`/`set_permissions` a `base.mode`; releer; si sigue distinto, **error visible**, no declarar éxito. Con `NONE` en la ruta rename, el modo se conservó en todos los casos 0600/0640/0644/0755; la lógica es una red de seguridad. Si el grupo no se conservó (solo ocurre con REPLACE_DESTINATION), tratarlo igual como error.
7. Actualizar `base` con el etag **devuelto** por `replace_contents` (es igual al consultado, 1b) y con hash de `bytes`; marcar limpio.
8. **Backups**: `make_backup=false` siempre (8a); `true` crearía `a.env~` con secretos (8b), contra TD §8.2/PRD §8.4.
9. **Atributos extendidos / SELinux**: la ruta rename **pierde xattr y la etiqueta SELinux personalizada** (9a) y la reemplaza por la del directorio. Para `.env` en `$HOME` es benigno (etiqueta por defecto ya correcta), pero ACL POSIX y xattr de usuario no se conservan. Decisión necesaria: documentarlo como limitación v0.1 (recomendado) o copiar xattr con `getxattr`/`setxattr` en la verificación posterior. ACLs no se probaron (ver §6).
10. **Hard links (D-34)**: por defecto Gio **no rompe** el enlace pero escribe en el sitio (7a: truncado + escritura, no atómico, ventana de pérdida y TOCTOU mayor 2g). Con `REPLACE_DESTINATION` sí es atómico pero rompe el enlace y pierde modo/grupo (7b, 3g). Recomendación: **tratarlos como los symlinks**: detectar `nlink>1` y bloquear el guardado normal con aviso explícito (opción A) y ofrecer Guardar como…; la opción B (documentar y permitir el guardado en el sitio) viola D-12.3. No hay trabajo extra relevante más allá del pre-vuelo del paso 2.
11. **Guardar como… (D-27)**: tras escribir un archivo nuevo con `PRIVATE` o con el modo que decida la política, `base` de la sesión pasa al nuevo archivo; al ser archivo nuevo, no hay fallback ni conflicto. Si la sesión lo hereda del original, aplicar modo del original (p. ej. 0600).

### Mapa a decisiones

| Decisión | Aplicación |
|---|---|
| D-12.1 TOCTOU | §3: hash+etag antes, replace con etag, verificación posterior, limitación documentada; no se promete exclusión mutua. El etag solo es mtime(ns) y es ciego a mtime restaurado → el hash es obligatorio |
| D-12.2 permisos | pasos 4 y 6: con NONE se conserva 0600/0640/0644/0755 y grupo; verificar siempre; error visible si no coincide (ocurre con REPLACE_DESTINATION y al perderse chown cuando el archivo es de otro usuario/grupo no propio, ver §6) |
| D-12.3 sin reemplazo atómico | paso 2: bloquear y ofrecer Guardar como… cuando el pre-vuelo prevea la ruta en el sitio (dir sin escritura 5a/5d, nlink>1 7a, symlink 6a, chown imposible) |
| D-34 enlaces duros | confirmado empíricamente: por defecto no se rompen pero se escriben en el sitio; con REPLACE_DESTINATION se rompen. Recomendación: tratarlos como symlinks (aviso/bloqueo) |
| D-25 hash | independiente del spike; el spike confirma que el hash es necesario (1f/2e), a decidir en ENV-T026 |
| D-27 Guardar como | paso 11 |

## 5. Implicaciones para ENV-T027 (lectura) y ENV-T028

- ENV-T027 debe capturar en `base`: hash del contenido, `etag::value`, `unix::mode`, `unix::uid`, `unix::gid`, `unix::inode`, `unix::nlink`, y si es symlink (`standard::is-symlink`, `standard::symlink-target`).
- ENV-T028 debe incluir pruebas de integración para cada fila relevante de §2 (tempdir real, no mocks) y no depender de `REPLACE_DESTINATION`.
- `FakeStorage` (T026) debe poder simular: WRONG_ETAG, dir no escribible, nlink>1, symlink, modo cambiado.

## 6. Qué no se pudo probar y por qué

- **Propietario distinto (uid ajeno)** y grupo no perteneciente al usuario: requiere root o `CAP_CHOWN`. Por diseño (fuente, líneas ~1055-1090): Gio intenta `fchown(tmp, uid, gid)` y `fchmod`; si fallan **y** el temporal queda con uid/gid/modo distintos del original, **cae a escritura en el sitio** (si el usuario tiene permiso de escritura sobre el archivo) en lugar de renombrar. Solo se pudo probar el grupo suplementario propio (3k/3l). No se verificó el fallback por chown fallido; el pre-vuelo del paso 2 cubre este caso por diseño.
- **ACL POSIX** (`setfacl`/`getfacl`) y flags `chattr` (inmutable/append-only): no probados. Se espera pérdida de ACL en ruta rename (el temporal es nuevo) y fallo con `chattr +i`.
- **Portal Flatpak / FUSE document portal** y backends GVfs (sftp, smb, nfs): no disponibles en este entorno. La conclusión de este spike aplica solo a archivo local; el entorno Flatpak es ENV-T054.
- **Fallo a mitad de escritura** (disco lleno, crash) en la ruta en el sitio: solo se demostró que el archivo queda **truncado a 0 bytes desde el open** (5d); no se inyectó un fallo real de `write`. No se midió el tamaño real de la ventana TOCTOU en tiempo.
- **Etags en NFS/btrfs con compresión/reflink**: no relevantes localmente; solo btrfs y tmpfs fueron ejecutados.
- **`getfattr` de ACL/capabilities**: solo `user.*` y `security.selinux`.
- Gio de otras versiones (el Flatpak runtime 26.08 podría traer otra GLib): el formato del etag (`sec:usec:nsec`) y el comportamiento de fallback deben re-verificarse en el runtime objetivo en ENV-T054/T082.

## 7. Resumen ejecutivo

1. El etag es mtime con precisión ns: detecta cambios de contenido, reescrituras idénticas (falso positivo benigno) y touch; **no** detecta contenido cambiado con mtime restaurado ni cambios de permisos. Hace falta hash de contenido además del etag.
2. WRONG_ETAG funciona, pero se evalúa **solo al abrir el stream**; hay una ventana TOCTOU real hasta el `rename` en la que un cambio externo se pierde sin error (demostrado).
3. Con rename (nlink==1, dir escribible, no symlink) los modos 0600/0640/0644/0755 y el grupo propio **se conservan** con `NONE`; `REPLACE_DESTINATION` los pierde.
4. 0444 falla con `PermissionDenied` (aceptable, no se elude el solo lectura).
5. **Riesgo P0 confirmado**: dir padre sin permiso de escritura (y symlink, y hard link) ⇒ Gio escribe **en el sitio** tras `ftruncate`: no atómico, sin señal en la API. Hay que bloquear con pre-vuelo.
6. Symlink: Gio escribe el target en el sitio y conserva el enlace (colgante: crea el target). Hard link: se conserva el enlace pero en el sitio.
7. `make_backup=false` no deja residuos; xattr y etiqueta SELinux personalizados se pierden en la ruta rename.

## 8. Revisión de seguridad posterior (2026-11-22)

La revisión de seguridad de este informe (fuente de GLib 2.88.4 y pruebas con python-gi) corrige la §4. Los detalles y criterios están en la nota de [ENV-T028](../BACKLOG.md#env-t028):

- **§4 paso 4 es incorrecto:** `replace_contents` cierra el stream tras un error de escritura y GLib renombra el temporal truncado sobre el original. Hay que usar `File::replace` con `Cancellable`, cancelar ante cualquier error y cerrar después.
- **`PRIVATE` siempre:** con `NONE`, el temporal se crea con `0666 & ~umask` antes del `fchmod`.
- **§4 paso 2:** la condición correcta es uid propio **y** gid en los grupos del proceso.
- **No cubierto en §2:** un archivo borrado tras la comprobación se recrea ignorando el etag.
- **§1.1:** `O_NOFOLLOW` solo detecta el symlink; ante `ELOOP` Gio reabre siguiendo el enlace.
