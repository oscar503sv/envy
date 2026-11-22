# ENV-T054 — Spike: acceso a carpetas, persistencia de permisos y CLI en Flatpak

## 0. Entorno

| Dato | Valor |
|---|---|
| Sesión | GNOME en Wayland, Fedora 44, kernel 7.3.4, usuario no root (uid 1000) |
| Flatpak | 1.18.4 |
| Runtime | `org.gnome.Platform//51` (gtk 4.24.1, GLib 2.90.1) + `rust-stable//26.08` (rustc 1.98.0), como fija D-06 |
| Comparación nativa | gtk 4.22.5, GLib 2.88.4, rustc 1.95.0 |
| Prototipo | App desechable `io.github.oscar503sv.EnvySpike` (gtk4 0.11.5, gio 0.22.11), fuera del repositorio (`~/.cache/t054-spike`); no se versiona |
| finish-args | `--share=ipc --socket=wayland --socket=fallback-x11 --device=dri`; **ningún `--filesystem`** |
| Evidencia | Log de la app con marca de tiempo (solo rutas, nombres, tamaños, etags, modos y códigos de error; nunca contenido) y registro horario de las acciones hechas desde el host |

Dos rondas: (1) automatizada, sin diálogo del portal (reenvío por CLI, reapertura, revocación, hermanos); (2) manual en la sesión real del usuario, con el diálogo del portal y un script que hizo las ediciones desde el host apuntando la hora de cada una. La evidencia de la primera ronda manual se perdió (la guía limpiaba el log antes de copiarlo); se repitió con el script guiado y es la que se cita.

## 1. Matriz de resultados

| # | Caso | Resultado | Evidencia |
|---|---|---|---|
| 1 | `FileDialog::select_folder` vía portal | **OK.** Devuelve `/run/user/1000/doc/<id>/t054-test` (FUSE del document portal). `query_info("xattr::document-portal.host-path")` devuelve la ruta real del host (`/home/<user>/t054-test`) | `chosen: path=…/doc/4gI1…/t054-test`, `host-path xattr = /home/…/t054-test` |
| 2 | Enumerar la carpeta y un nivel de subcarpeta | **OK.** 4 entradas, 2 `.env*`, y `sub/.env.local` visible (la concesión de carpeta es recursiva) | `enum: total=4 env_matches=2 subdirs=1`, `enum/sub: … .env.local` |
| 3a | Escribir con `replace_contents` | **OK.** Etag devuelto = etag consultado después; modo 644 conservado; el cambio llega al host | `rw[replace_contents]: write OK … mode_before=644 mode_after=644` |
| 3b | Escribir con `replace(etag, false, PRIVATE)` (estrategia D-12) | **OK**, igual que 3a; sin `.goutputstream-*` residuales | `rw[replace_stream]: write OK …` |
| 4 | Monitor de las escrituras de la propia app | **OK.** `GInotifyFileMonitor`; eventos `Created/Changed/Renamed .goutputstream-* → .env` | `monitor[dir]: event=Renamed … other=Some(".env")` |
| 5 | **Monitor ante ediciones hechas desde el host** | **FALLA: cero eventos.** Desde el host, a las 12:11:08–12:11:16: `echo >> .env`, `touch new.txt`, `mv`, `rm`, reemplazo atómico de `.env`. El log no tiene ningún evento entre el arranque del monitor (12:11:05) y la siguiente acción de la app (12:11:42). Reproducido también en la ronda automatizada | `host-actions.txt` frente a `t054.log` |
| 5b | Releer tras la edición externa | **OK.** La relectura ve el cambio (`.env` pasa de 73 a 4 bytes; el etag cambia) | `enum: … .env … size=4`, `etag_before=1795371076:…` |
| 6 | Reabrir con la referencia guardada en un proceso nuevo | **OK** sin diálogo | `startup-reopen[path]: REOPEN SUCCESS` |
| 6b | Reabrir tras reiniciar `xdg-document-portal` y `xdg-desktop-portal` | **OK**, mismo id | `REOPEN SUCCESS` a las 12:11:57 |
| 6c | Reabrir tras cerrar sesión o reiniciar | **No probado.** El almacén de permisos es persistente en disco y el id sobrevivió al reinicio de los portales, así que se espera éxito; queda para ENV-T055 | – |
| 7 | **Carpeta borrada y recreada en el host** (`rm -rf` + `cp -r`) | La concesión está ligada al **inodo** (`dev 46, ino 2599140` en la tabla `documents`). Al recrear la carpeta, el diálogo creó un id nuevo (`4gI1…`, ino 2600872) y la entrada vieja (`S_MU…`) quedó **sin permisos para la app**. Una referencia guardada con el id viejo deja de funcionar | `flatpak permissions documents` antes/después |
| 8a | Revocar con `flatpak document-unexport <ruta>` | **OK**: la reapertura falla con `kind=NotFound` | `revoke3.txt`: `REOPEN FAILED` |
| 8b | Revocar con `flatpak permission-reset <app>` | **No revoca**: la reapertura sigue funcionando | `revoke.txt` |
| 8c | `flatpak permission-remove documents <id> <app>` | `No entry` sobre un documento creado por `--file-forwarding` (esa concesión no figura por app en la tabla). No se probó sobre un documento creado por el diálogo, que sí tiene la fila por app. La revocación desde Ajustes de GNOME tampoco quedó registrada | `revoke2.txt`; recuerdo del usuario (`No entry`) |
| 9 | CLI: `flatpak run --file-forwarding <app> @@ <archivo> @@` | **OK**: llega `/run/user/1000/doc/<id>/.env` con su xattr de ruta del host. Con una carpeta, también (ronda automatizada) | `open[0]: path=…/doc/pTJb…/.env` |
| 10 | CLI con ruta plana, sin reenvío | **Falla** con `NotFound`: la ruta del host no existe dentro del sandbox | `open[0]: query ERR kind=NotFound` |
| 11 | Abrir un archivo suelto con `FileDialog::open`: ¿se ven sus hermanos? | **No.** El documento solo contiene ese archivo; `.env.example` no es visible | `sibling.txt` |

## 2. Conclusiones

1. **Workspace con portales es viable** (D-19 opción a): seleccionar carpeta, enumerar recursivamente, leer, escribir con la estrategia de D-12 y reabrir sin diálogo funcionan sin ningún `--filesystem`.
2. **Los cambios externos no se notifican dentro del sandbox.** El FUSE del document portal no propaga inotify de escrituras hechas fuera de él. Afecta a cualquier archivo abierto por el portal, no solo a Workspace: el monitoreo de ENV-005 no se puede basar solo en `GFileMonitor` dentro de Flatpak. Hace falta comprobar por sondeo (etag/mtime + hash, ENV-T027) al recuperar el foco de la ventana, con un temporizador moderado mientras el documento está abierto, y siempre antes de guardar (esto último ya lo exige D-12).
3. **La referencia persistente es la ruta `/run/user/UID/doc/<id>/<nombre>`**, que es estable entre ejecuciones y entre reinicios del portal. Para mostrarla se usa el xattr `document-portal.host-path`; nunca se muestra la ruta `/run/user/…`.
4. **La referencia se rompe si la carpeta se borra y se recrea** (por ejemplo `rm -rf` + `git clone`). La app debe validar la referencia al arrancar y, si falla, ofrecer «Volver a elegir la carpeta» en vez de un error genérico.
5. **Abrir un archivo suelto no da acceso a sus hermanos.** Comparar con `.env.example` (ENV-T065) o descubrir otros `.env` exige que el usuario elija la carpeta.
6. **CLI en Flatpak:** solo funciona con reenvío de archivos. El `.desktop` debe usar `Exec=envy @@u %U @@` (o el equivalente que genere `flatpak-builder`), y la documentación CLI debe explicar `flatpak run --file-forwarding` (D-20).
7. **Revocación:** `document-unexport` revoca de forma fiable; `permission-reset` no. La UI debe tratar `NotFound` en una referencia guardada como «acceso retirado».

## 3. Propuesta de finish-args mínimos

| Permiso | Motivo |
|---|---|
| `--share=ipc` | Requisito de X11/MIT-SHM para `fallback-x11`; estándar en apps GTK |
| `--socket=wayland` | Ventana en Wayland |
| `--socket=fallback-x11` | Solo se usa si no hay Wayland |
| `--device=dri` | Renderizado por GPU de GTK 4 (opcional: sin él, GTK usa software) |

Sin `--filesystem`, sin `--share=network`, sin `--talk-name`. Los portales FileChooser y Documents no necesitan permisos declarados. No se pide `--filesystem=home` porque el caso de uso se cubre con portales y Flathub lo cuestiona en un editor de secretos.

## 4. Verificación tras la revisión de seguridad (2026-11-22)

La revisión de seguridad planteó tres hipótesis sin probar: F1, que las comprobaciones de D-12/D-34 no se pueden observar a través del FUSE del portal; F2, que el FUSE sirva datos viejos ante ediciones del host en el mismo inodo; y F3, que el sondeo actualice la línea base. F1 y F2 se midieron con exportaciones `flatpak document-export` (mismo almacén que el diálogo) y un shell dentro del sandbox; F3 es una regla de diseño.

**F2: descartado.** Desde el host se añadió una línea en el sitio, se sobrescribió con el mismo tamaño (`dd conv=notrunc`) y se truncó y reescribió, sin cambiar de inodo. El sondeo dentro del sandbox, cada 0,2 s, vio cada cambio (tamaño, mtime en ns y sha256) en la primera lectura posterior. No hay caché de atributos ni de páginas que oculte cambios.

**F1: matriz.** Columnas: archivo suelto exportado frente a archivo dentro de una carpeta exportada; «guardar» es `gio save` (`g_file_replace` sin etag) desde el sandbox.

| Caso en el host | Qué ve el sandbox | Resultado de guardar | ¿Detectable en el pre-vuelo de D-12? |
|---|---|---|---|
| 0600, grupo primario | modo 600, uid/gid 1000 | rename (inodo nuevo), modo conservado | sí |
| 0640, grupo suplementario (gid 10) | gid **65534** (grupo no mapeado en el user namespace) | **en el sitio** (inodo igual): `fchown` al gid falla y GIO cae al truncado no atómico; el gid se conserva en el host | sí: el gid no es el del usuario, así que «gid conservable» falla y se bloquea Guardar |
| Enlace duro (`nlink=2`) dentro de carpeta | `nlink=2` | en el sitio (cambian los dos nombres) | sí, `nlink > 1` (D-34) |
| Symlink dentro de carpeta | `lstat` muestra el symlink | GIO sigue el enlace y escribe en el sitio en el destino (como ENV-T025 6a) | sí, `lstat` (D-34) |
| **Symlink exportado como archivo suelto** | el portal **resuelve** el enlace: el documento contiene el destino con su propio nombre (`real.env`); el nombre del enlace no existe (`NotFound`) | se edita el destino; el symlink del host se conserva | **no**: la app no puede saber que el usuario eligió un enlace. Escribir en el nombre inexistente deja un temporal huérfano `.xdp-<nombre>-XXXXXX` en el directorio del host |
| ACL POSIX | `system.posix_acl_access` visible | – | sí |
| Etiqueta SELinux | siempre `fusefs_t` (sintética) | – | **no**: la etiqueta real no es observable |
| Padre 0777 sin sticky, archivo suelto | el padre es el directorio sintético del documento (`700`) | rename correcto | **no**: el padre real no es visible. En una carpeta exportada sí se ve el modo real |

**Consecuencias para D-19:**

- Las comprobaciones de D-12/D-34 siguen siendo válidas en Flatpak para modo, uid/gid, `nlink`, symlinks dentro de carpetas y ACL.
- El gid 65534 debe tratarse como «no conservable» (bloquear Guardar y ofrecer Guardar como), igual que cualquier gid ajeno.
- No son verificables en Flatpak: la etiqueta SELinux, el directorio padre de un archivo suelto y si un archivo suelto era un symlink. Se documentan como limitaciones del modo Flatpak. El comportamiento resultante es benigno: el portal escribe sobre el destino real por la ruta de rename y el symlink del host se conserva. Lo que se pierde es el aviso de PRD §8.5.
- **F3** se incorpora como regla: el sondeo solo marca «cambiado externamente»; la línea base (etag/hash) solo cambia al recargar, al fusionar o tras un guardado propio verificado. Nunca se recarga sola con cambios sin guardar, y el sondeo se pausa durante un guardado propio.

## 5. Pendiente para ENV-T055 / ENV-T081

- Persistencia tras cerrar sesión y tras reiniciar (6c).
- Revocación desde Ajustes de GNOME > Aplicaciones y con `permission-remove` sobre un documento creado por el diálogo (8c).
- Coste y frecuencia del sondeo en carpetas con muchos archivos.
