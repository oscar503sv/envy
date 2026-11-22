# Especificación de sintaxis dotenv de Envy — v0.1

| Campo | Valor |
|---|---|
| Estado | v0.1, normativo; desarrolla D-11 (docs/planning/DECISIONS.md) |
| Tarea | ENV-T006 |
| Consumidores | ENV-T007 (fixtures), ENV-T008 (modelo), ENV-T010/T011 (parser), ENV-T012 (serializador), ENV-T014 (`ValueCodec`) |
| Alcance | Un **subconjunto** de dotenv, definido por este documento. **No** pretende compatibilidad universal con ninguna biblioteca dotenv ni con shells. |
| Precedencia | PRD US-02 y D-11 > este documento > TECHNICAL_DESIGN §5. Este documento solo concreta detalles de implementación que D-11 no fija. Si una regla de aquí parece contradecir D-11 o el PRD, **no se resuelve en silencio**: se eleva como pregunta abierta (sección 12) o como discrepancia (sección 12.1). |

## 0. Principio rector

> Todo lo dudoso se resuelve a `Raw` y se preserva **byte a byte**. Envy nunca expande variables, nunca ejecuta nada y nunca repara ni reformatea regiones que el usuario no ha tocado.

Consecuencias directas: un archivo sin editar se serializa idéntico a su origen (TD §5.2 invariante 1); un nodo `Raw` no admite operaciones estructurales (TD §5.2 invariante 4); ante cualquier ambigüedad entre "asignación editable" y `Raw`, gana `Raw`.

## 1. Terminología

| Término | Definición |
|---|---|
| Byte | Unidad de preservación. La fidelidad se mide en bytes, nunca en cadenas normalizadas. |
| Espacio en blanco (`WS`) | Exclusivamente U+0020 (espacio) y U+0009 (tabulador). Cualquier otro carácter, incluidos U+000B, U+000C, U+00A0 y U+FEFF, **no** es blanco (ver Q-02). |
| Terminador de línea | `LF` (`\n`), `CRLF` (`\r\n`) o `CR` (`\r` no seguido de `\n`). Una secuencia `\r\n` es siempre un único `CRLF`. |
| Línea física | Bytes desde el inicio del cuerpo o desde justo después de un terminador, hasta el siguiente terminador (excluido) o EOF. Una última línea sin terminador es una línea física válida. Tras un terminador final no hay línea física vacía adicional. |
| Nodo | Unidad del modelo (`EnvNode`). Cubre un rango contiguo de bytes del origen: una o más líneas físicas completas, **incluido su terminador**. |
| `Assignment` | Nodo de asignación `KEY=value`. Puede abarcar varias líneas físicas (valor entre comillas multilínea). |
| `Comment` | Línea cuyo primer carácter no blanco es `#`. |
| `Blank` | Línea vacía o solo con `WS`. Es el `NodeKind::Blank` de TD §5.1; D-11 regla 10 lo llama `Empty` (ver Q-01). |
| `Raw` | Cualquier región no reconocida. Se preserva sin interpretar. |
| Clave (`key`) | Identificador que cumple `^[A-Za-z_][A-Za-z0-9_]*$` (ASCII). |
| Valor (`value`) | Los bytes **editables** de una asignación, definidos por estilo en la sección 5. |
| Prefijo / sufijo | Bytes de una asignación antes / después del valor, preservados en una edición mínima (TD §5.2 invariante 6). El terminador de línea se considera una tercera parte, tras el sufijo. |
| `QuoteStyle` | `Unquoted`, `Single`, `Double` (TD §5.1). |
| `NeedsConversion` | Resultado de `encode` cuando el valor nuevo no se puede escribir de forma segura en el estilo actual; indica el estilo destino (siempre `Double` en v0.1) y exige confirmación explícita del usuario. |
| Estilo dominante | Estilo de terminador (`LF`/`CRLF`/`CR`) usado para contenido nuevo (SYN-06). |

Notación: en los ejemplos, `\n`, `\r`, `\t` entre backticks son los bytes LF, CR, TAB; los espacios son espacios. **Excepción:** dentro de las comillas dobles de los ejemplos de SYN-23, SYN-24 y SYN-37, y en la tabla de escapes de 5.3, cada `\X` son los **dos caracteres literales** del archivo (barra invertida y letra), porque es lo que esas reglas describen. Cualquier otra barra invertida literal se escribe con una letra que no sea `n`, `r` ni `t`.

## 2. Reglas a nivel de documento

1. **UTF-8.** Para edición estructurada, el contenido (tras el BOM) debe ser UTF-8 válido. Si no lo es, el documento se abre en **modo protegido de solo lectura**: no se parsea en nodos, no se ofrece edición y no se reescribe nunca (SYN-01).
2. **BOM** (D-11 regla 7). Los bytes `EF BB BF` en el offset 0 son un atributo del documento (`bom: bool`), **fuera de cualquier nodo**. Se serializa primero si `bom` es `true`; nunca se añade ni se quita. Un `EF BB BF` en cualquier otra posición es contenido ordinario (SYN-02, Q-14).
3. **Terminadores.** Cada nodo conserva sus propios terminadores. Los archivos con terminadores mixtos se preservan sin normalizar (SYN-03).
4. **Última línea sin terminador** es válida y se preserva sin añadirle uno (SYN-04). Un archivo vacío tiene cero nodos (SYN-05).
5. **Estilo dominante** (D-11 regla 8, SYN-06). Para el contenido nuevo o pegado se cuenta cada terminador del documento (LF solos, CRLF, CR solos; ver Q-09 sobre los terminadores dentro de valores). El estilo con mayor recuento es el dominante; en empate gana el estilo del **último** terminador del documento; sin terminadores, `LF`. El texto pegado se normaliza al estilo dominante, incluidos los saltos dentro de valores multilínea. La normalización se hace al pegar, no en `encode`.
6. **Newline final al insertar** (D-11 regla 9, SYN-07). Al insertar una línea al final:
   - archivo vacío (o solo BOM): la línea nueva termina en `LF`;
   - el archivo acababa en terminador: la línea nueva termina en el terminador dominante;
   - el archivo no acababa en terminador: se añade el terminador dominante a la antigua última línea y la línea nueva queda **sin** terminador.
7. **Newline final al eliminar** (SYN-08). Al eliminar la última línea de un archivo cuya última línea no tenía terminador, se quita el terminador de la nueva última línea (el archivo sigue sin newline final). Si se elimina la única línea, el resultado es vacío. Si el archivo sí acababa en terminador, no se toca nada más.
8. **Líneas intermedias.** Una línea insertada fuera del final termina en el terminador dominante.

## 3. Clasificación de líneas (algoritmo ordenado)

El parser recorre el cuerpo (tras el BOM) de izquierda a derecha. En cada posición `pos` que inicia una línea física aplica **en este orden**; el primer paso que coincide decide. `eol` es el fin del contenido de la línea física (sin terminador).

```text
C0  Si la línea contiene U+0000                         -> Raw (una línea)        [Q-15]
C1  Si el contenido [pos, eol) es vacío o solo WS        -> Blank
C2  Si el primer carácter no blanco es '#'               -> Comment
C3  Intentar cabecera de asignación sobre el contenido:
        ^ WS* ( 'export' WS+ )? KEY WS* '=' 
    (primero con el grupo `export`; si lo que sigue no cumple `KEY WS* '='`,
     reintentar sin él, de modo que `export=1` es la clave `export`).
    Si no coincide                                       -> Raw (una línea)
    Si coincide: sea W el tramo de WS tras '=' y v el primer carácter tras W.
      - v es '"'  -> escaneo Double (sección 5.3)
      - v es '\'' -> escaneo Single (sección 5.2)
      - otro (o fin de línea) -> Unquoted (sección 5.1)
    El escaneo puede dar: Assignment | Raw multilínea | Raw hasta EOF
C4  (inalcanzable por construcción; cualquier fallo de C3 ya es Raw)
```

Notas:
- `export` exige uno o más `WS` (espacios o tabuladores) antes de la clave; `exportKEY=1` es la clave `exportKEY`. `EXPORT KEY=1` no es prefijo (distingue mayúsculas): es `Raw`.
- `export KEY` sin `=` es `Raw`. `KEY` sin `=` es `Raw`.
- Claves con `.`, `-`, espacios internos, comienzo en dígito, vacías o con caracteres no ASCII son `Raw`.
- Las líneas `Raw` de una sola línea son un nodo por línea física. Las regiones `Raw` multilínea (comillas mal cerradas) son **un único nodo** (Q-13).
- Tras un nodo, el parser continúa en la línea física siguiente al último byte (terminador incluido) del nodo.
- Un `Comment` puede tener `WS` inicial; el `#` puede no estar en la columna 1 (D-11 regla 5).

## 4. Anatomía de una asignación

Todo nodo `Assignment` descompone sus bytes en **prefijo + valor + sufijo + terminador**, sin solapes ni huecos (invariante 1):

```text
 prefijo                      valor        sufijo              terminador
|  export KEY = "          |  contenido  |" # nota          |  \n
```

- `value_span` (TD §5.1) cubre solo el **valor**. `is_export` indica presencia de `export`. `is_multiline` es `true` si el valor contiene algún terminador real en el origen.
- El terminador es el de la **última** línea física del nodo y puede no existir (EOF).
- Una edición del valor sustituye únicamente `value_span`; prefijo, sufijo y terminador permanecen intactos (SYN-41). Si el valor editado es igual al mostrado, no se genera reemplazo.

## 5. Reglas de valor

Sea `v` el primer carácter tras `=` y el tramo `W` de `WS`.

### 5.1 Sin comillas (`Unquoted`) — D-11 reglas 3 y 5

- **Se entra aquí** si `v` no es `"` ni `'`. Una comilla que aparece más adelante dentro del valor (`KEY=ab"cd`) es contenido literal (SYN-34).
- **Fin del valor.** El contenido va desde `v` hasta el primero de: fin de línea, o un `#` cuyo carácter anterior es `WS` (espacio o tabulador), donde ese `WS` puede ser el último de `W`. Un `#` sin `WS` inmediatamente anterior pertenece al valor (`PASS=ab#cd`, `URL=https://x/#frag`, `KEY=#x`) (SYN-20).
- Los `WS` finales **no** forman parte del valor; los `WS` internos sí (`KEY=a  b` -> `a  b`).
- Una barra invertida no es especial; no hay continuación de línea (Q-16).
- **Bytes.**
  - Prefijo: desde el inicio del nodo hasta `v` (incluye `WS` inicial, `export`, clave, `WS`, `=` y `W`).
  - Valor: `[v, e)`, con `e` el fin del valor tras recortar `WS` finales.
  - Sufijo: desde `e` hasta `eol` (`WS` finales y comentario final, si hay).
- **Valor vacío** (`KEY=`, `KEY=   `, `KEY= # nota`) es válido y editable (SYN-21). El valor es el rango vacío en `v`. Si **no** hay comentario final, `W` pertenece al prefijo (`KEY=  ` -> prefijo `KEY=  `, valor vacío, sufijo vacío). Si hay comentario final, `W` pertenece al **sufijo** y el valor vacío queda justo tras `=` (así `KEY= # n` editado a `x` da `KEY=x # n`, sin pegar el valor al comentario) (Q-12).
- Ejemplos: `KEY=value # nota` -> valor `value`, sufijo ` # nota`. `KEY=a\t# c` -> valor `a`, sufijo `\t# c`. `KEY=a #b #c` -> valor `a`, sufijo ` #b #c`.

### 5.2 Comillas simples (`Single`) — D-11 reglas 2 y 4

- Se entra cuando `v` es `'`. El contenido es **literal**: no existen escapes; la barra invertida es un byte más.
- **Cierre.** Es la primera `'` posterior a la apertura, aunque esté en otra línea física. Los terminadores intermedios forman parte del valor (valor multilínea, SYN-26).
- **Bytes.** Prefijo: hasta la comilla de apertura inclusive. Valor: bytes entre comillas, sin ellas. Sufijo: comilla de cierre, el resto de la línea física donde cierra (SYN-27/28).
- Si no hay comilla de cierre hasta EOF: sección 5.6.

### 5.3 Comillas dobles (`Double`) — D-11 reglas 1 y 4

- Se entra cuando `v` es `"`.
- **Cierre.** Se recorre el contenido desde la apertura: una barra invertida `\` **consume el carácter siguiente** (sea cual sea, incluida `"` o un terminador; `CRLF` cuenta como un solo carácter) y ese par nunca cierra. El cierre es la primera `"` no consumida (SYN-25). Una `\` al final del archivo sin carácter siguiente deja la comilla sin cerrar.
- **Bytes.** Igual que en 5.2: el valor son los bytes crudos entre comillas (escapes sin decodificar); el cierre y lo que sigue en esa línea son sufijo.
- **Tabla de escapes (decodificación para mostrar):**

| En el archivo | Se muestra |
|---|---|
| `\\` | `\` |
| `\"` | `"` |
| `\n` | LF |
| `\r` | CR |
| `\t` | TAB |
| `\X` con cualquier otro `X` (incl. `\$`, `\u`, `\0`, `\'`, `\` + terminador) | **literal**: se mantienen **ambos** caracteres, `\` y `X` (SYN-24) |

- "Literal" significa que la secuencia no se interpreta ni se descarta ninguno de sus caracteres. Si `X` es un carácter multibyte, se conserva entero. Si `X` es un terminador real, se muestra `\` seguido de un salto de línea (con la normalización de 6.1).
- Un salto de línea real dentro de las comillas se conserva como salto real (SYN-35).
- `$VAR`, `${VAR}`, `$(cmd)` son texto literal (sección 5.7).

### 5.4 Valores multilínea

- Un valor entre comillas continúa en la línea física siguiente mientras no se encuentre el cierre. El nodo abarca desde la línea de apertura hasta el final de la línea física del cierre, terminador incluido.
- Reglas de cierre: `Single` sin escapes (primera `'`); `Double` con la regla de `\` de 5.3.
- Un `CR` solitario cuenta como terminador dentro de un valor entre comillas (necesario para archivos solo-CR; ver Q-04).
- `is_multiline = true` si el valor contiene al menos un terminador.
- Dentro de un valor multilínea, las líneas **no** se clasifican: aunque parezcan `KEY=1` o `# x`, son contenido (SYN-35).
- Excepción (aclaración de §11 y Q-15, 2026-11-22): U+0000 en cualquier línea física que cubra el nodo, incluida la parte posterior al cierre, convierte el nodo entero en `Raw` desde la línea de apertura hasta la de cierre.

### 5.5 Qué puede seguir a la comilla de cierre — D-11 regla 4

En el resto de la línea física donde cierra, tras la comilla de cierre solo se aceptan:
- nada, o
- `WS*` y fin de línea, o
- `WS+` seguido de `#` y cualquier texto (comentario final). Se exige al menos un `WS` antes del `#` (Q-03).

Cualquier otro byte (`KEY="a"b`, `KEY="a" b`, `KEY="a"#c`, `KEY='a''b'`) convierte el nodo en **`Raw`**. El nodo `Raw` abarca desde la línea de apertura hasta el final de la línea física del cierre, terminador incluido (SYN-27).

### 5.6 Comillas sin cerrar — D-11 regla 6

Si desde la apertura no existe comilla de cierre válida hasta EOF, **desde el inicio de la línea de apertura hasta EOF** todo es **un único nodo `Raw`**, con bytes preservados y diagnóstico visible, con este texto exacto (N = número de línea física, base 1, de la apertura):

```text
Comillas sin cerrar en la línea N; el resto del archivo no es editable hasta corregirlo
```

Solo el primer caso de comillas sin cerrar importa (todo lo posterior ya es `Raw`) (SYN-30).

Consecuencia que se acepta: una comilla huérfana puede "cerrarse" accidentalmente mucho más adelante con otra comilla del mismo tipo y absorber líneas intermedias en un valor multilínea o en un `Raw` por 5.5 (`unclosed-quote-middle.env`, SYN-31, Q-08).

### 5.7 Expansiones — D-11 regla 1

`$VAR`, `${VAR}` y `$(cmd)` son **texto literal** en cualquier estilo; Envy nunca expande ni ejecuta (PRD §8). Si el valor mostrado contiene `${`, la UI avisa de que otras herramientas sí expanden `${…}` (alcance del aviso por estilo: Q-11). El aviso no altera el modelo (SYN-32).

### 5.8 Comentarios

- Línea de comentario: primer carácter no blanco `#`; se preserva literal, incluido su `WS` inicial (SYN-10).
- Comentario final: solo tras `WS` (Unquoted, 5.1) o tras la comilla de cierre (5.5).
- `#` dentro de comillas es **siempre** contenido (SYN-29).

### 5.9 Claves duplicadas

Cada ocurrencia es un nodo independiente con su propio `id`; el parser no deduplica ni valida unicidad (TD §5.2 invariante 3, SYN-17).

## 6. ValueCodec (base de ENV-T014)

`decode(style, bytes) -> String` (bytes crudos del `value_span` -> valor mostrado) y `encode(style, &str, ctx) -> Encoded | NeedsConversion{to: Double}`, con `ctx = {estilo dominante, nodo multilínea?}`.

### 6.1 Decode

| Estilo | Regla |
|---|---|
| `Unquoted` | Identidad (SYN-36). |
| `Single` | Identidad, salvo que cada terminador interno (`LF`, `CRLF`, `CR`) se muestra como un único LF (Q-05). |
| `Double` | Tabla de 5.3 y cada terminador interno real se muestra como un único LF. |

### 6.2 Encode

**`Double`** (SYN-37):
- `\` -> `\\`; `"` -> `\"`; CR -> `\r`.
- LF -> si el nodo ya era multilínea: terminador real del estilo dominante; si no (nodo de una sola línea o nuevo): `\n` (Q-06).
- Cualquier otro carácter (incluido TAB) se escribe tal cual.
- Propiedad exigida: `decode(encode(v)) == v` para todo `v`. No se exige `encode(decode(raw)) == raw`; por eso una edición solo reescribe el valor si el valor mostrado cambió (SYN-41). Un `\X` desconocido mostrado como `\X` se reescribe como `\\X` al editar ese valor.
- `Double` nunca devuelve `NeedsConversion`.

**`Single`** (SYN-38; D-11 regla 2):
- Si el valor contiene `'` -> `NeedsConversion` hacia `Double` (solo con confirmación).
- Si contiene CR -> `NeedsConversion` hacia `Double` (un CR real sería un terminador; ver Q-04/Q-06).
- Si no, se escribe tal cual; cada LF se escribe como terminador real dominante (permite valor multilínea).

**`Unquoted`** (SYN-39; D-11 regla 3):
- Si el valor contiene LF o CR -> `NeedsConversion` hacia `Double`.
- Si empieza o termina por `WS` -> `NeedsConversion` hacia `Double`.
- Si no, se escribe tal cual.

**Guardas de ida y vuelta** (SYN-40, extensión de D-11 regla 3, Q-07): tras codificar, se compone `prefijo + bytes + sufijo` y se reclasifica la línea. Si el resultado no es el mismo `Assignment`, estilo y valor, el resultado es `NeedsConversion` hacia `Double`. Casos típicos: valor `Unquoted` con `WS` seguido de `#` (se leería como comentario), valor que empieza por `"` o `'` (se leería como entrecomillado), valor que empieza por `#` cuando el prefijo termina en `WS`.

**Confirmación.** `NeedsConversion` nunca se aplica sin confirmación explícita; sin ella, la edición se cancela y el nodo no cambia. La conversión cambia las comillas del prefijo y el sufijo (`KEY=a b` -> `KEY="a b"`); el comentario final y el terminador se conservan.

## 7. Tabla de reglas

Los ejemplos escapan los no imprimibles. "Resultado" indica tipo de nodo y valor mostrado.

| ID | Regla | Ejemplo | Resultado | Fixture |
|---|---|---|---|---|
| SYN-01 | Contenido no UTF-8 válido: modo protegido, solo lectura, jamás se reescribe | bytes `FF FE 4B 3D 80` | Documento protegido; sin nodos; bytes intactos | `invalid-utf8.bin` |
| SYN-02 | BOM `EF BB BF` en offset 0 es el atributo `bom` del documento; fuera de nodos; se serializa primero; nunca se añade ni quita | `\xEF\xBB\xBFA=1\n` | `bom = true`; `Assignment` `A` = `1` | `utf8-bom.env` |
| SYN-03 | Cada nodo conserva su terminador (`LF`, `CRLF`, `CR`); `\r\n` es un solo `CRLF`; archivos mixtos se preservan | `A=1\r\nB=2\nC=3\r` | 3 `Assignment` con terminadores `CRLF`, `LF`, `CR` | `mixed-newlines.env`, `only-cr.env` |
| SYN-04 | Última línea sin terminador es válida y se preserva sin añadirlo | `A=1\nB=2` | 2 `Assignment`; el segundo sin terminador | `no-trailing-newline.env` |
| SYN-05 | Archivo vacío: cero nodos | (0 bytes) | Sin nodos | `empty.env` |
| SYN-06 | Estilo dominante por recuento; empate: último terminador; sin terminadores: `LF`; pegado normalizado a ese estilo | `A=1\r\nB=2\nC=3\r\n` | Dominante `CRLF` | `mixed-newlines.env`, `only-cr.env` |
| SYN-07 | Insertar al final: acababa en newline -> la nueva lleva terminador; no acababa -> terminador a la antigua última y la nueva sin él; vacío -> `LF` | `A=1` + insertar `B=2` | `A=1\nB=2` (dominante `LF`, sin terminador final) | `no-trailing-newline.env`, `empty.env`, `plain.env` |
| SYN-08 | Eliminar la última línea de un archivo sin newline final: se quita el terminador de la nueva última línea | `A=1\nB=2` - eliminar `B` | `A=1` | `no-trailing-newline.env` |
| SYN-09 | Línea vacía o solo con `WS` es `Blank` (otros blancos Unicode no: Q-02) | `   \t\n` | `Blank` | `comments-spacing.env` |
| SYN-10 | Primer carácter no blanco `#`: `Comment`, preservado literal con su `WS` inicial | `  # texto\n` | `Comment` | `comments-spacing.env` |
| SYN-11 | `KEY=value` con clave `^[A-Za-z_][A-Za-z0-9_]*$` es `Assignment` `Unquoted`; `=` extra pertenece al valor | `A=b=c\n` | `Assignment` `A` = `b=c` | `plain.env` |
| SYN-12 | `WS` antes de la clave y alrededor de `=` permitido y preservado en prefijo | `  KEY \t=  v\n` | `Assignment` `KEY` = `v`; prefijo `  KEY \t=  ` | `comments-spacing.env`, `plain.env` |
| SYN-13 | `export` + `WS+` + clave válida: `Assignment` con `is_export`; `export` y su `WS` son prefijo; nunca se ejecuta | `export KEY=v\n`, `export\tKEY=v\n`, `export export=1\n` | `Assignment` `is_export` | `export-prefix.env`, `export-variants.env` |
| SYN-14 | `export` mal formado es `Raw`: sin `=`, sin `WS`, otra capitalización | `export KEY\n`, `EXPORT KEY=1\n`, `export\n` | `Raw` | `export-variants.env` |
| SYN-15 | Clave fuera de `^[A-Za-z_][A-Za-z0-9_]*$` (`.`, `-`, dígito inicial, espacio interno, vacía, no ASCII): `Raw` | `my-key=1\n`, `a.b=2\n`, `1A=3\n`, `=x\n`, `claveñ=1\n` | `Raw` | `raw-keys.env` |
| SYN-16 | Línea sin `=` o con sintaxis ajena (heredoc, shell, `KEY: v`, backticks): `Raw` | `KEY: v\n`, `cat <<EOF\n`, `` `cmd` ``  | `Raw` | `unknown-syntax.env`, `raw-keys.env` |
| SYN-17 | Claves duplicadas: nodos independientes, orden físico preservado | `A=1\nA=2\n` | 2 `Assignment` `A` | `duplicate-keys.env` |
| SYN-18 | `Unquoted`: valor hasta fin de línea; `WS` internos son valor; `WS` finales van al sufijo | `K=a  b  \n` | `K` = `a  b`; sufijo `  ` | `plain.env` |
| SYN-19 | Comentario final si `#` va precedido de espacio o tabulador; no entra en el valor; sufijo preservado | `K=v # n\n`, `K=v\t# n\n` | `K` = `v`; sufijo ` # n` / `\t# n` | `inline-comments.env` |
| SYN-20 | `#` sin `WS` previo pertenece al valor | `PASS=ab#cd\n`, `URL=http://x/#f\n`, `K=#x\n` | valores `ab#cd`, `http://x/#f`, `#x` | `inline-comments.env` |
| SYN-21 | `KEY=` es valor vacío editable; con comentario, `W` va al sufijo | `K=\n`, `K=  \n`, `K= # n\n` | `K` = `` (vacío) | `plain.env`, `inline-comments.env` |
| SYN-22 | `Single`: contenido literal, sin escapes, `\` es un byte más | `K='C:\dir\x'\n` | `Single`, valor `C:\dir\x` (las barras se conservan) | `quoted-single.env` |
| SYN-23 | `Double`: escapes `\\ \" \n \r \t` se decodifican para mostrar | `K="a\tb\"c"\n` | `Double`, valor `a<TAB>b"c` | `quoted-double.env` |
| SYN-24 | `Double`: cualquier otro `\X` se muestra literal (se mantienen `\` y `X`) | `K="a\qb\$"\n` | valor `a\qb\$` | `quoted-double.env` |
| SYN-25 | `Double` multilínea: `\` consume el siguiente carácter; cierra la primera `"` no consumida | `K="a\"\nb"\n` (con `\n` = LF real tras `\"`) | `Double` multilínea, valor `a"<LF>b` | `quoted-multiline.env` |
| SYN-26 | `Single` multilínea: sin escapes, cierra la primera `'` | `K='a\nb'\n` (LF real) | `Single` multilínea, valor `a<LF>b` | `quoted-multiline.env` |
| SYN-27 | Tras la comilla de cierre solo `WS` y comentario opcional; otro byte: `Raw` desde la línea de apertura hasta la línea del cierre | `K="a"b\n`, `K="a" x\n` | `Raw` | `inline-comments.env`, `unclosed-quote-middle.env` |
| SYN-28 | Comentario tras la comilla de cierre, con al menos un `WS`; sufijo preservado | `K="a" # n\n`, `K='a'\t# n\n` | `K` = `a`; sufijo con cierre y comentario | `inline-comments.env` |
| SYN-29 | `#` dentro de comillas es contenido | `K="a # b"\n`, `K='#x'\n` | `K` = `a # b`, `#x` | `hash-in-quotes.env` |
| SYN-30 | Comilla sin cerrar: `Raw` único desde la línea de apertura hasta EOF + diagnóstico exacto | `A=1\nB="x\nC=2\n` | `A` `Assignment`; `B..EOF` `Raw`; "Comillas sin cerrar en la línea 2; el resto del archivo no es editable hasta corregirlo" | `unclosed-quotes.env` |
| SYN-31 | Comilla huérfana cerrada por otra posterior: se aplican las reglas sin heurística (multilínea o `Raw` por SYN-27) | `A="x\nB=1\nC="y"\nD=2\n` | `A..C` un `Raw` (tras `"` queda `y"`); `D` `Assignment` | `unclosed-quote-middle.env` |
| SYN-32 | `$VAR`, `${VAR}`, `$(cmd)` son texto literal; nunca se expanden; la UI avisa por `${…}` | `K=${HOME}/$USER\n`, `K=$(id)\n`, `K="${A}"\n` | valores literales | `literal-expansion.env` |
| SYN-33 | Valores con Unicode (UTF-8 válido) se preservan; claves no ASCII son `Raw` (SYN-15) | `K=café 世界\n` | `K` = `café 世界` | `unicode-values.env` |
| SYN-34 | Solo se abre comilla si es el primer carácter del valor; una comilla posterior en un `Unquoted` es literal | `K=ab"cd\n`, `K=it's\n` | `Unquoted`, valores `ab"cd`, `it's` | `plain.env` |
| SYN-35 | Dentro de un valor multilínea las líneas no se clasifican; sus terminadores forman parte del valor y se conservan | `K="a\nB=1\n# x\n"\n` (LF reales) | un solo `Assignment` multilínea | `quoted-multiline.env`, `only-cr.env` |
| SYN-36 | decode `Unquoted`: identidad; `Single`: identidad con terminadores a LF; `Double`: tabla 5.3 con terminadores a LF | `K='a\r\nb'` | valor `a<LF>b` | `plain.env`, `quoted-single.env`, `quoted-double.env` |
| SYN-37 | encode `Double`: `\`->`\\`, `"`->`\"`, CR->`\r`, LF->`\n` o terminador real si el nodo es multilínea; nunca `NeedsConversion` | valor `a"b\c` | `a\"b\\c` | `quoted-double.env`, `quoted-multiline.env` |
| SYN-38 | encode `Single`: `'` o CR en el valor -> `NeedsConversion` a `Double` (con confirmación); si no, literal | valor `it's` | `NeedsConversion(Double)` | `quoted-single.env` |
| SYN-39 | encode `Unquoted`: LF/CR o `WS` inicial/final -> `NeedsConversion` a `Double` | valor `a\nb`, valor ` a` | `NeedsConversion(Double)` | `plain.env` |
| SYN-40 | Guarda de ida y vuelta: si la línea codificada no se reclasifica igual, `NeedsConversion` a `Double` | valor `a #b`, valor `"x"` en `Unquoted` | `NeedsConversion(Double)` | `inline-comments.env`, `hash-in-quotes.env` |
| SYN-41 | Edición mínima: solo se reemplaza el valor; prefijo, sufijo y terminador intactos; sin cambio de valor no hay reemplazo | `K = "a" # n\r\n` cambiar a `b` | `K = "b" # n\r\n` | `plain.env`, `comments-spacing.env`, `inline-comments.env` |
| SYN-42 | `Raw` no admite edición estructural y se preserva byte a byte | `<<EOF\n`, `KEY\n` | `Raw` sin cambios | `unknown-syntax.env`, `raw-keys.env` |

Total: **42 reglas** (SYN-01 a SYN-42).

## 8. Trazabilidad

### 8.1 TD §5.3 -> SYN

| Fila de TD §5.3 | SYN |
|---|---|
| `KEY=value` | SYN-11, SYN-18, SYN-34 |
| `KEY = value` | SYN-12 |
| `export KEY=value` | SYN-13, SYN-14 |
| `KEY='value'` | SYN-22, SYN-38 |
| `KEY="value"` | SYN-23, SYN-24, SYN-37 |
| Valor entre comillas en varias líneas físicas | SYN-25, SYN-26, SYN-35 |
| Comentario de línea | SYN-10 |
| Comentario al final | SYN-19, SYN-20, SYN-28, SYN-29 |
| `KEY=` vacío | SYN-21 |
| Líneas vacías / `LF`, `CRLF`, `CR`, mixtos | SYN-03, SYN-04, SYN-05, SYN-09 |
| `$OTHER`, `${OTHER}`, `$(command)` literal | SYN-32 |
| Claves con guiones/puntos u otra gramática | SYN-15, SYN-16 |
| Heredoc, shell avanzado, sintaxis no familiar | SYN-16, SYN-42 |
| UTF-8 no válido | SYN-01 |

### 8.2 D-11 -> SYN

| Regla de D-11 | SYN |
|---|---|
| 1 Comillas dobles (escapes, saltos reales, expansiones literales) | SYN-23, SYN-24, SYN-25, SYN-32, SYN-35, SYN-37 |
| 2 Comillas simples (literales, `NeedsConversion`) | SYN-22, SYN-26, SYN-38 |
| 3 Sin comillas (fin de valor, sufijo, `NeedsConversion`) | SYN-18, SYN-19, SYN-39, SYN-40 |
| 4 Tras la comilla de cierre | SYN-27, SYN-28 |
| 5 Comentarios | SYN-10, SYN-19, SYN-20, SYN-29 |
| 6 Comillas sin cerrar | SYN-30, SYN-31 |
| 7 BOM | SYN-02 |
| 8 Salto de línea de contenido nuevo | SYN-06 |
| 9 Newline final | SYN-07, SYN-08 |
| 10 Otras reglas (`export`, espacios, `KEY=`, claves, líneas vacías) | SYN-09, SYN-11 a SYN-16, SYN-21 |
| PRD US-02, TD §5.2 (invariantes 1, 4, 6; UTF-8) | SYN-01, SYN-41, SYN-42 |

### 8.3 Puntos que podrían parecer abiertos y D-11 ya resuelve

- Tabulador antes del `#` final: **sí** inicia comentario (regla 5, "espacio o tabulador").
- `export` seguido de tabulador: **válido** (regla 10, "espacio(s)/tab").
- `WS` antes de `#` en líneas de comentario: **permitido** (regla 5, "primer carácter no blanco").

## 9. Fixtures

ENV-T007 crea los archivos; este documento fija su propósito y las reglas que cubren. Directorio propuesto: el de TD §14.1. Cada fixture debe incluir al menos un caso positivo por regla asignada. Los de TD §14.1 son los 16 primeros.

| Fixture | Propósito | Reglas SYN |
|---|---|---|
| `empty.env` | 0 bytes | SYN-05, SYN-07 |
| `plain.env` | Asignaciones simples, `=` en el valor, `KEY=` vacío, comillas literales en medio, espacios internos y finales | SYN-07, SYN-11, SYN-12, SYN-18, SYN-21, SYN-34, SYN-36, SYN-39, SYN-41 |
| `quoted-single.env` | Valores `Single`: contenido literal, barras, `#` y `$` | SYN-22, SYN-36, SYN-38 |
| `quoted-double.env` | `Double` con los cinco escapes, `\X` desconocido y comilla escapada | SYN-23, SYN-24, SYN-36, SYN-37 |
| `quoted-multiline.env` | Valores `Single` y `Double` multilínea; cierre con escapes; `\` seguida de un terminador real dentro de `Double` (se consume y se muestra `\` + salto); líneas internas con aspecto de `KEY=` o `#` | SYN-25, SYN-26, SYN-35, SYN-37 |
| `comments-spacing.env` | Comentarios con y sin `WS` inicial, `Blank` con `WS`, espacios y tabuladores alrededor de `=` | SYN-09, SYN-10, SYN-12, SYN-41 |
| `inline-comments.env` | Comentarios finales con espacio y tabulador, `#` sin espacio previo en el valor, comentario tras comillas, basura tras comillas (`Raw`), valores que no pueden ser `Unquoted` | SYN-19, SYN-20, SYN-21, SYN-27, SYN-28, SYN-40, SYN-41 |
| `duplicate-keys.env` | La misma clave varias veces | SYN-17 |
| `mixed-newlines.env` | `LF`, `CRLF` y `CR` en un mismo archivo | SYN-03, SYN-06 |
| `no-trailing-newline.env` | Última línea sin terminador | SYN-04, SYN-07, SYN-08 |
| `utf8-bom.env` | BOM seguido de contenido | SYN-02 |
| `unicode-values.env` | Valores UTF-8 multibyte, emoji, caracteres combinados | SYN-33 |
| `unknown-syntax.env` | Heredoc, shell, `KEY: v`, backticks y una línea con U+0000 (regla C0, Q-15) | SYN-16, SYN-42 |
| `export-prefix.env` | `export KEY=v` básico y con espacios | SYN-13 |
| `invalid-utf8.bin` | Bytes no UTF-8 válidos | SYN-01 |
| `unclosed-quotes.env` | Comilla de apertura sin cierre hasta EOF, con líneas válidas antes | SYN-30 |
| `only-cr.env` | Archivo cuyos únicos terminadores son `CR`, incluido un valor multilínea | SYN-03, SYN-06, SYN-35 |
| `export-variants.env` | `export` con tabulador, varios espacios, `export export=1`, `export KEY`, `EXPORT KEY=1`, `exportKEY=1` | SYN-13, SYN-14 |
| `unclosed-quote-middle.env` | Comilla huérfana a mitad del archivo cerrada por otra posterior (absorción y `Raw` por basura tras cierre) | SYN-27, SYN-31 |
| `hash-in-quotes.env` | `#` dentro de comillas simples y dobles, también precedido de espacio | SYN-29, SYN-40 |
| `literal-expansion.env` | `$VAR`, `${VAR}`, `$(cmd)` en los tres estilos | SYN-32 |
| `raw-keys.env` | Claves con `.`, `-`, dígito inicial, espacio interno, vacía, no ASCII, línea sin `=` | SYN-15, SYN-16, SYN-42 |

Cobertura: cada SYN-01..SYN-42 aparece en al menos un fixture (verificación mecánica recomendada en ENV-T007: extraer los IDs de esta tabla y comparar con la sección 7).

## 10. Centinela de pruebas

Los fixtures que contengan secretos falsos usan **exactamente** el valor `ENVY_TEST_SENTINEL_9F2C` (decisión de proyecto). Cualquier aparición de ese valor en logs, mensajes de error, salida `Debug` o pánico es una **fuga**. Los tests de fidelidad y de codec deben comprobar que no aparece fuera de los buffers de datos. Los diagnósticos (p. ej. el de comillas sin cerrar) citan solo el número de línea, nunca el contenido.

## 11. Fuera de alcance y siempre `Raw`

- Heredocs, sustituciones y sintaxis de shell (`$(…)`, backticks, `if`, `;`, `&&`, redirecciones).
- Expansión de variables de cualquier tipo, incluidos valores por defecto `${A:-b}`.
- Otras extensiones de dialecto: `KEY: value` estilo YAML, comillas de backtick, continuación con `\` al final de línea, `KEY = value` con claves entre comillas, `export -p`, comandos `unset`.
- Claves con caracteres no ASCII, `.`, `-`, espacios o que empiezan por dígito.
- Líneas con U+0000 (Q-15).

## 12. Preguntas abiertas

**Aprobadas por el usuario el 2026-11-21**: los 16 valores por defecto son normativos y este documento los implementa. Se conservan las opciones descartadas como contexto.

- **Q-01 — Nombre `Blank` / `Empty`.** D-11 regla 10 dice `Empty`; TD §5.1 define `NodeKind::Blank`. Opciones: (a) `Blank` en todo el código y actualizar D-11; (b) `Empty`. **Defecto: `Blank` (mismo tipo de nodo; solo cambia el texto de D-11).**
- **Q-02 — Qué es "blanco".** Opciones: (a) solo espacio y tabulador; (b) también U+000B, U+000C, U+00A0 y Unicode `White_Space`. **Defecto: (a); una línea con solo otros caracteres de blanco es `Raw`, no `Blank`.**
- **Q-03 — `#` pegado a la comilla de cierre** (`K="a"#c`). Opciones: (a) `Raw`; (b) comentario. **Defecto: (a), se exige al menos un `WS` antes del `#`, coherente con la regla 5.**
- **Q-04 — `CR` solitario dentro de un valor multilínea entre comillas.** Opciones: (a) cuenta como terminador (necesario para archivos solo-CR); (b) hace `Raw` el nodo si el archivo mezcla estilos; (c) es contenido literal. **Defecto: (a), por coherencia con la clasificación de líneas; los bytes se preservan en cualquier caso. Si el usuario prefiere lo más conservador, (b).**
- **Q-05 — Visualización de terminadores internos.** Opciones: (a) mostrar siempre LF y escribir con el estilo dominante al editar; (b) conservar el terminador de cada salto. **Defecto: (a), alineado con la regla 8 (incluye saltos dentro de valores multilínea). Implica que editar un valor multilínea con terminadores mixtos los uniformiza.**
- **Q-06 — Escritura de LF en `Double` y `\X` desconocido.** Opciones: (a) LF como `\n` en nodos de una línea y terminador real en nodos multilínea; (b) siempre `\n`; (c) siempre terminador real. Además, un `\X` desconocido se reescribe `\\X` al editar. **Defecto: (a); la reescritura de `\X` se acepta porque garantiza `decode(encode(v)) == v`.**
- **Q-07 — Disparadores adicionales de `NeedsConversion` en `Unquoted`.** D-11 regla 3 solo nombra saltos de línea y espacios inicial/final. Se añade la guarda de ida y vuelta (`WS` + `#`, comilla inicial, `#` inicial tras `WS` del prefijo). Opciones: (a) añadirlas; (b) escribir el valor tal cual (corrompe). **Defecto: (a).**
- **Q-08 — Comilla huérfana que se cierra por accidente** (SYN-31). Opciones: (a) aplicar las reglas sin heurística, con indicador visible de multilínea; (b) si un valor multilínea contiene alguna línea que se clasificaría como `Assignment`, tratar toda la región como `Raw`. **Defecto: (a) literal, porque (b) contradice D-11 regla 1; pero es el punto de mayor riesgo para el usuario y conviene un aviso en la UI.**
- **Q-09 — Alcance del recuento del estilo dominante.** Opciones: (a) todos los terminadores del archivo, incluidos los de `Raw` y los de dentro de valores; (b) solo los de fin de nodo. **Defecto: (a).**
- **Q-10 — Insertar al final cuando la última región es `Raw` por comillas sin cerrar.** Añadir un terminador a esa región alteraría su contenido. Opciones: (a) bloquear "añadir al final" y permitir insertar solo antes de la región; (b) permitirlo. **Defecto: (a).**
- **Q-11 — Alcance del aviso por `${…}`.** Opciones: (a) avisar en `Unquoted` y `Double`, no en `Single`; (b) avisar en los tres estilos. **Defecto: (b), por ser lo más prudente; (a) reduciría ruido, pero otras herramientas se comportan distinto.**
- **Q-12 — Asignación de `WS` en valor vacío** (sección 5.1). Opciones: (a) `W` al prefijo salvo que haya comentario final, caso en que va al sufijo; (b) `W` siempre al prefijo; (c) siempre al sufijo. **Defecto: (a), para que `K= # n` editado dé `K=x # n`.**
- **Q-13 — Granularidad de regiones `Raw` multilínea** (comillas sin cerrar, basura tras cierre). Opciones: (a) un solo nodo; (b) un nodo `Raw` por línea. **Defecto: (a), por el invariante 2 de TD §5.2 y para que una región no se parta.**
- **Q-14 — BOM.** Un segundo BOM o uno que no está en el offset 0 es contenido ordinario (en una clave la vuelve `Raw`). Un archivo de solo BOM cuenta como vacío para la regla 9. **Defecto: sí, ambas cosas.**
- **Q-15 — Líneas con U+0000.** Es UTF-8 válido. Opciones: (a) `Raw`; (b) contenido normal. **Defecto: (a).**
- **Q-16 — Barra invertida final en `Unquoted`** (`K=a\`). Opciones: (a) literal, sin continuación; (b) continuación de línea estilo shell. **Defecto: (a); (b) sería sintaxis de shell, fuera de alcance.**

### 12.1 Posibles discrepancias D-11 / TD

1. **`Empty` vs `Blank`**: D-11 regla 10 vs TD §5.1 (Q-01). Es nominal, no semántico.
2. **BOM**: D-11 regla 7 exige `bom: bool` fuera de los nodos; TD §5.1 `EnvDocument` no tiene ese campo y TD §5.2 invariante 8 solo dice que el flujo "puede conservar" el BOM. Hay que añadir `bom` a `EnvDocument` y hacer que los spans de los nodos empiecen tras el BOM.
3. **Granularidad de `value_span`**: TD §5.1 define un solo `value_span` y D-11 exige prefijo y sufijo con comentario final. Este documento los hace derivables (prefijo = inicio de nodo hasta `value_span.start`; sufijo = `value_span.end` hasta el final del nodo menos el terminador), pero TD no define el terminador como campo; ENV-T008 debe concretarlo.
4. **`NeedsConversion` en TD §5.3**: TD no lo nombra (habla de "solicitar conversión explícita"); D-11 sí. Sin contradicción, solo vocabulario.
