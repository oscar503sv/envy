# Matriz de guardado seguro — ENV-T067

La suite nativa usa archivos reales y datos sintéticos marcados con `SENTINEL`. Comprueba los bytes completos y busca respaldos y temporales en la carpeta. Las filas manuales identifican el entorno de verificación pendiente. Las filas con herramientas o permisos opcionales registran una omisión si el entorno carece de ellos.

| Fila | Resultado esperado | Cobertura |
|---|---|---|
| Archivo 0600 | Conserva el modo, publica un inodo nuevo y permite encadenar guardados. | `tests/save_safety.rs::chained_saves_preserve_mode_and_replace_inode` |
| Archivo 0644 | Conserva el modo, publica un inodo nuevo y limpia el estado de edición pendiente. | `tests/save_safety.rs::chained_saves_preserve_mode_and_replace_inode` |
| Archivo 0400 | `Storage(ReadOnly)`, bytes intactos. | `tests/save_safety.rs::read_only_file_and_parent_refuse_save` |
| Padre 0555 | `SaveAsRequired { reason: AtomicReplaceUnsupported }`. El pre-vuelo deja desactivada la capacidad de reemplazo atómico cuando el padre no admite crear entradas. | `tests/save_safety.rs::read_only_file_and_parent_refuse_save` |
| Documento symlink | La sesión rechaza la apertura con `Symlink`. | `tests/save_safety.rs::symlink_document_and_dangling_save_as_are_refused` |
| Cambio de contenido justo antes de guardar y sin evento | `Storage(Conflict)`, disco y edición local intactos. Sobrescribir exige confirmar la versión revisada. | `tests/save_safety.rs::unobserved_disk_changes_require_review_before_overwrite` |
| `chmod`, cambio de mtime o contenido con mtime restaurado sin evento | Conflicto con historial intacto. Se comprueba la versión completa, incluido el hash. | `tests/save_safety.rs::unobserved_disk_changes_require_review_before_overwrite` |
| Reemplazo externo por rename, al estilo de Vim | El cambio de inodo provoca conflicto aunque coincidan los bytes. | `tests/save_safety.rs::unobserved_disk_changes_require_review_before_overwrite` |
| Archivo eliminado sin evento | La sesión queda en Missing, conserva la edición y no recrea el archivo. La regresión detectó que se devolvía `ReadOnly`. El servicio ahora relee en ese caso y devuelve `NotFound` si confirma la eliminación. | `tests/save_safety.rs::save_after_unobserved_deletion_reports_missing_or_conflict_without_recreating` |
| Escritura parcial con `RLIMIT_FSIZE` | El error conserva el original completo y elimina el temporal. | `tests/gio_storage.rs::partial_write_preserves_original_and_removes_temporary_file` |
| Soltar el stream sin cerrar | El original se conserva si se abandona la escritura. En la implementación actual, `PendingFile::drop` retira el temporal ante un error. | `tests/gio_storage.rs::partial_write_preserves_original_and_removes_temporary_file`. El comportamiento del stream GIO anterior se estudió en ENV-T025. |
| Borrado entre pre-vuelo y guardado | Conflicto sin recreación. | `src/infrastructure/gio_storage.rs::deletion_after_preflight_is_a_conflict_without_recreation` |
| Sustitución por archivo o symlink después del pre-vuelo | La comprobación final rechaza la sustitución antes de publicar. | Manual en el adaptador nativo, ENV-T075, con una pausa justo antes de `PreparedSave::check_base`. `tests/gio_storage.rs::changed_deleted_and_symlink_destinations_are_never_written` cubre la sustitución antes de entrar al pre-vuelo. |
| Directorio de grupo 0770/0750 y 2770/2750 | Se rechazan los padres escribibles por el grupo. En los demás se conserva el gid de archivo con y sin setgid. | `tests/save_safety.rs::supplementary_group_directories_preserve_file_group`, con gid primario cuando falta un grupo suplementario apto. |
| UID o GID ajeno | El pre-vuelo impide publicar si el propietario es ajeno o el grupo no se puede conservar. | Manual en contenedor nativo, ENV-T075, con privilegios para preparar propietarios y grupos ajenos. |
| Padre con ACL por defecto, o archivo con ACL de acceso | Rechazo antes de publicar. | `tests/gio_storage.rs::refuses_access_and_default_acl_before_publication`, `tests/gio_storage.rs::create_new_refuses_default_acl`. Requiere `setfacl` y soporte en el montaje. |
| Etiqueta SELinux propia | En el host se rechaza el reemplazo si el temporal tendría una etiqueta diferente. | `tests/gio_storage.rs::refuses_custom_selinux_label_before_publication`. Fila no ejecutada en CI (runner sin SELinux); verificación en Fedora nativo, ENV-T075. |
| `chattr +i` y `+a` | Se rechaza publicar y se conserva el archivo. | Manual en Linux nativo, ENV-T075, sobre un montaje compatible con los atributos y con privilegios para establecerlos. |
| Directorio sticky | El propietario autorizado puede reemplazar el archivo con un inodo nuevo. | `tests/gio_storage.rs::replaces_with_verified_hash_and_new_inode_in_sticky_directory` |
| Directorio sticky 1777 de otro UID | Verificar el rechazo seguro para un padre ajeno no confiable y que no cambia el archivo. | Manual en Linux nativo, ENV-T075, con segundo UID y montaje que respete sticky. |
| Guardar como sobre symlink colgante | `Symlink`. El destino del enlace sigue ausente. | `tests/save_safety.rs::symlink_document_and_dangling_save_as_are_refused` |
| Umask 000, 002 y 022 | Guardar conserva 0644 y Guardar como crea en 0600. | `tests/save_safety.rs::save_and_save_as_preserve_modes_under_three_umasks` |
| Temporal creado en 0600 | El descriptor nace privado y se mantiene así durante la escritura. | `src/infrastructure/gio_storage.rs::deletion_after_preflight_is_a_conflict_without_recreation` comprueba que el modo inicial no concede bits fuera de 0600. `tests/save_safety.rs::sigkill_during_atomic_replace_preserves_complete_bytes` verifica 0600 en los temporales observados para un original 0600. |
| SIGKILL entre creación y rename | El disco contiene exactamente la versión vieja o la nueva. Al abrir se limpia cualquier staging que se haya envejecido. | `tests/save_safety.rs::sigkill_during_atomic_replace_preserves_complete_bytes`, `tests/gio_storage.rs::stale_staging_cleanup_only_removes_old_single_link_regular_files`. El momento de SIGKILL se muestrea al azar, principalmente durante la escritura del temporal. El intervalo entre `fsync` y `renameat` queda sin cobertura determinista. |
| `O_TMPFILE` + `linkat` | Evaluar compatibilidad y permisos de publicación como posible mejora. | Manual en Flatpak real, ENV-T082. Requiere FUSE del document portal. |
| Edición del host menos de 1 s antes de guardar | Detectar el cambio pese al caché de atributos y a la ausencia de eventos del host. | Manual en ENV-T082 sobre FUSE. ENV-T054 midió la caché y la ausencia de eventos. `tests/file_monitor.rs::focus_and_timer_poll_and_drop_cancels_pending_work` cubre el sondeo nativo. |
| `renameat` aplicado con respuesta perdida | Si el destino identifica al archivo publicado, se informa `PostWriteVerificationFailed { reason: Identity }`. | Manual con inyección de respuesta perdida en FUSE o FS de red. Validación del portal en ENV-T082. |
| Inestabilidad del padre durante la verificación | Puede notificarse `Identity` con contenido completo. Se conserva la edición local para revisión. | `tests/gio_storage.rs::permission_race_preserves_complete_file_with_and_without_write_limit`, `src/services/document_service/tests.rs::identity_warning_requires_reviewed_overwrite_and_content_warning_can_retry` |
| ENOSPC real | Un error durante el staging conserva el original completo. | Manual en Linux nativo, ENV-T075, con un montaje pequeño dedicado. `RLIMIT_FSIZE` cubre un error de escritura distinto. |
| Fallback `linkat` sin `RENAME_NOREPLACE` | Publicación exclusiva, sin sobrescribir si otro proceso crea el destino. | Manual en FS nativo sin el flag en ENV-T075 y en el portal en ENV-T082. `tests/gio_storage.rs::create_new_never_overwrites_a_racing_creator` cubre la exclusión sin forzar el fallback. |
| Nombre de usuario `.envy-<16hex>.tmp` antiguo | La limpieza borra el archivo propio de un solo enlace si supera los diez minutos. | `tests/save_safety.rs::open_removes_user_file_with_reserved_staging_name_after_ten_minutes` |
| Bucle de symlinks en ancestros | Guardar se bloquea y `create_new` devuelve `Io` con `FilesystemLoop`. | `tests/save_safety.rs::ancestor_symlink_loop_refuses_save_and_create_new` |
| Falsa alarma residual por carrera de chmod del padre | Puede aparecer `Identity` aun con bytes correctos. La frecuencia aproximada histórica de 0,1 % no es un umbral de CI. | `tests/gio_storage.rs::permission_race_preserves_complete_file_with_and_without_write_limit` cubre la integridad durante la carrera. |
| Guardar como New desde sesión en conflicto | La sesión adopta el archivo nuevo y deja intacto el original externo. | `tests/save_safety.rs::save_as_checks_aliases_and_switches_only_after_publication` |
| Guardar como New sobre un archivo existente, y ReplaceExisting sobre un destino distinto | New devuelve `AlreadyExists`. ReplaceExisting publica los bytes revisados y cambia la sesión al destino. | `tests/save_safety.rs::save_as_checks_aliases_and_switches_only_after_publication` |
| ReplaceExisting sobre enlace duro o alias del padre | `InvalidSaveAs` al identificar el mismo documento por dispositivo e inodo. | `tests/save_safety.rs::save_as_checks_aliases_and_switches_only_after_publication` |
| ReplaceExisting sobre un alias tras reemplazar el original al estilo de Vim | `InvalidSaveAs` al releer el original y detectar que el destino comparte el inodo nuevo. | `tests/save_safety.rs::save_as_rejects_alias_of_externally_replaced_original` |

## Cómo ejecutar

En Linux, con Rust 1.95.0 y las dependencias de compilación del README disponibles:

```sh
envy_test_cache=$(mktemp -d "$HOME/.cache/envy-t067.XXXXXXXX")
export TMPDIR="$envy_test_cache" CARGO_TARGET_DIR="$envy_test_cache/target"
cargo fmt --check
cargo clippy --offline --all-targets --all-features --locked -- -D warnings
cargo test --offline --locked --all-targets
cargo test --offline --locked --doc
for run in 1 2 3; do cargo test --offline --locked --test save_safety || break; done
rm -rf -- "$envy_test_cache"
```

## Riesgos residuales

- **R-01 (TOCTOU).** La comprobación final y el rename son operaciones distintas. Un escritor concurrente puede actuar en ese intervalo.
- **Identidad del padre.** En QA de ENV-T030 se observó aproximadamente un 0,1 % de avisos `Identity` bajo cambios concurrentes de permisos, con contenido correcto.
- **Nombre reservado.** La limpieza trata el patrón exacto de staging como reservado incluso si el usuario eligió ese nombre.
- **Portal.** Falta validar en ENV-T082 la lectura de ACL y `create_new` para archivos sueltos y carpetas compartidas, la identidad devuelta por Guardar como y las primitivas de publicación. D-19 admite que SELinux no es observable en rutas del portal y que el padre de una exportación suelta puede no ser verificable.
