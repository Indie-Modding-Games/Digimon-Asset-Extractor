# Plan de remediación de integridad

**Proyecto:** Digimon Asset Extractor
**Fuentes de verdad:** [Revisión de integridad](../quality/INTEGRITY_REVIEW.md) y [FEATURES](../../FEATURES.md)
**Fecha:** 2026-10-21
**Estado:** 0/10 hallazgos cerrados · 103 pruebas en verde (`cargo test --all-targets`)
**Plan complementario:** [Plan de soporte de formatos](FORMAT_SUPPORT_PLAN.md) (este plan no añade formatos; cierra huecos de los ya anunciados)

---

## Objetivo

Cerrar los hallazgos de la revisión de integridad y los puntos de «Pendiente de atención» de `FEATURES.md`, de forma que:

1. Ninguna salida del extractor o de los conversores afirme más de lo que hace (verificación, texturas, vídeo, geometría).
2. Los comandos de la CLI tengan un contrato verificable de salida, recursión y códigos de error.
3. Cada formato anunciado tenga una validación contra datos reales del juego, no solo pruebas sintéticas.
4. El proyecto sea reproducible (versiones de dependencias fijadas).

Criterio de cierre del plan: los 10 hallazgos de la tabla de inventario están en estado «Resuelto» o «Retirado con nota en la documentación», cada uno con su comprobación de aceptación ejecutada.

---

## Principios de ejecución

1. **Un ciclo por iteración.** Cada ciclo es un cambio pequeño, revisable y verificable por separado.
2. **Honestidad antes que funcionalidad.** Si una salida es aproximada, se etiqueta como tal en la CLI y en la documentación; no se anuncia soporte completo.
3. **Nada de valores inventados.** Hashes, tamaños y referencias solo se publican con una fuente comprobable; si no la hay, se retiran y se dice que no hay referencia.
4. **Prueba o justificación.** Cada ciclo incluye una prueba automatizada focalizada, o una nota explícita de por qué aún no es posible automatizarlo.
5. **No se rompe la extracción existente.** Los cambios en el extractor se validan contra el BIN real (`base/Digimon World Vice.bin`) antes de darse por cerrados.

### Limitación del entorno (afecta a los ciclos de comparación)

No hay compilador de C++ (`g++`/`cl`/`clang++`/`cmake`) en el `PATH`, así que la referencia `dw-engine` **no se puede ejecutar localmente** para generar goldens. Los ciclos que exigen contraste con la referencia se resuelven así:

- **Opción A (preferida):** se genera el golden en otra máquina con la referencia y se versiona como fixture en `tests/test_extract/`.
- **Opción B:** se valida contra el comportamiento real del juego (pistas de audio, scripts, vídeo) con criterios observables (p.ej. número de muestras por sector, longitud de los scripts, coherencia de los fotogramas).
- **Opción C:** se documenta la diferencia como pendiente y el hallazgo queda abierto.

---

## Inventario de hallazgos

| ID | Severidad | Hallazgo | Origen | Ciclo | Estado |
|---|---|---|---|---|---|
| H1 | Alta | Las referencias de Redump son marcadores y `Extractor::extract_file` no verifica nada | Revisión / FEATURES | 1.1 | Pendiente |
| H2 | Alta | `extract-files` ignora `--no-verify` (verificación fija a `false`) | Revisión / FEATURES | 1.2 | Pendiente |
| H3 | Media | `convert-tim` incumple su ayuda: `--recursive` sin uso, `--output` ambiguo, errores absorbidos con salida 0 | Revisión / FEATURES | 2.1 | Pendiente |
| H4 | Media | `parse_cue` parte la línea `FILE` por espacios: un BIN con espacios se trunca | Revisión | 1.3 | Pendiente |
| H5 | Media | Extracción de texturas MAP no implementada; OBJ es aproximado; `convert-map` no expone TFS | Revisión / FEATURES | 5.1, 5.2 | Pendiente |
| H6 | Media | `convert-str --extract-video` entrega bitstreams MDEC, no vídeo reproducible | Revisión / FEATURES | 3.2, 6.1 | Pendiente |
| H7 | Media | Las pistas STR no se pueden extraer del BIN con el extractor actual | Revisión | 3.1 | En validación: extracción y conversión reales funcionan; falta comparación independiente del WAV |
| H8 | Media | El soporte SCN solo está validado con scripts sintéticos | Revisión / FEATURES | 4.1 | Pendiente |
| H9 | Baja/Media | `Cargo.lock` está en `.gitignore`: las versiones resueltas pueden variar | Revisión / FEATURES | 7.1 | Pendiente |
| H10 | Media | `convert-audio` (WAV/SF2/MIDI) sin validar contra los bancos de sonido reales | FEATURES | 4.2 | Pendiente |

---

## Fase 1 — Integridad de la extracción (Alta)

### 1.1 Redump: referencias de marcador y verificación muerta

**Problema.** En `src/verification/mod.rs:54-66` las entradas de `SYSTEM.CNF` y `CHDAT/ALLTIM.TIM` llevan hashes de relleno: el MD5 `d41d8cd98f00b204e9800998ecf8427e` es el de un archivo vacío, el SHA256 `e3b0c442…b855` también, y el SHA256 de `SYSTEM.CNF` es una secuencia artificial. Además, `Extractor::extract_file` (`src/extractor/mod.rs:183`) no consulta `RedumpDatabase::verify` y marca todo como `NoReference`: la base de datos no verifica nada, y si se conectara tal cual daría estados engañosos (`Mismatch` sobre hashes falsos).

**Objetivo.** Que la verificación signifique algo o que no exista: ningún hash publicado sin fuente comprobable.

**Archivos.**
- `src/verification/mod.rs` — `RedumpDatabase::new`, `RedumpEntry`, `verify`.
- `src/extractor/mod.rs` — `extract_file`, `ExtractionFileResult`, `verify_formats`.
- `src/cli/mod.rs` — opciones `--manifest`, `--no-verify` y textos de ayuda.
- `docs/reference/CLI.md`, `docs/reference/FORMATS.md`, `FEATURES.md`, `README.md` — descripciones de la verificación.

**Pasos.**
1. Retirar las dos entradas de marcador de `RedumpDatabase::new()`.
2. Añadir carga de una base Redump real desde archivo (`redump.txt`/lista `sha256 + tamaño`), opcional por ruta, con un loader propio que no dependa de la red.
3. Decidir el comportamiento cuando no hay base: el estado `NoReference` es honesto y se documenta como tal; el manifiesto pasa a distinguir «sin referencia» de «verificado».
4. Si la base Redump se conecta a la extracción, hacerlo con la misma semántica de `--no-verify` (ciclos 1.1 y 1.2 comparten el interruptor).
5. Ajustar la documentación: describir la verificación como «integridad interna del manifiesto» mientras no haya fuente Redump.

**Comprobaciones de aceptación.**
- Prueba unitaria: `RedumpDatabase::new()` no contiene ninguna entrada cuyo `md5` sea el hash de archivo vacío ni cuyo `sha256` no sea hexadecimal de 64 caracteres.
- Prueba unitaria: un archivo sin entrada en la base devuelve `NoReference`; uno con entrada correcta devuelve `Verified`; uno con tamaño distinto devuelve `Mismatch`.
- Prueba unitaria del loader con una lista Redump de fixture (3 líneas) → verifica, no verifica y detecta `Mismatch`.
- `cargo test --all-targets` en verde.

**Decisiones pendientes.**
- ¿Se versiona una lista Redump parcial (solo los archivos extraídos de este juego) o se retira el concepto hasta disponer de la lista oficial?
- Si se conecta la verificación: ¿el fallo por `Mismatch` debe abortar la extracción o solo annotate el manifiesto?

---

### 1.2 `extract-files` ignora `--no-verify`

**Problema.** `Commands::ExtractFiles` declara `no_verify` (`src/cli/mod.rs:265`), pero `run_extract_files` (`src/cli/mod.rs:343`) no lo recibe y construye el `Extractor` con `verify_formats = false` fijo (`:353`, `:355`, `:357`). En `run_extract` el valor sí se propaga como `!no_verify` (`:263`). El resultado: la validación de formato queda desactivada en extracción selectiva con o sin la opción, y el usuario cree lo contrario.

**Objetivo.** Que `--no-verify` signifique lo que dice en los dos comandos de extracción.

**Pasos.**
1. Añadir el parámetro `no_verify: bool` a `run_extract_files` y propagar `!no_verify` a las tres construcciones del `Extractor`.
2. Extraer la construcción del `Extractor` a un helper (`build_extractor(output, force, verify_formats, config, preserve_chdat)`) para que el interruptor sea comprobable en una prueba unitaria.
3. Revisar qué hace `verify_formats` (validación de cabecera por formato) con los archivos que la extracción selectiva admite: si un archivo no tiene formato reconocible, el estado debe ser `format_valid = false` y no un error fatal.
4. Documentar en `docs/reference/CLI.md` la diferencia entre `extract` y `extract-files` en cuanto a verificación.

**Comprobaciones de aceptación.**
- Prueba unitaria del helper: con `verify_formats = true` y `false` el campo refleja el valor; el test falla si alguien vuelve a cablear `false` fijo.
- Prueba de integración ligera: `extract-files` con `--no-verify` sobre un fixture produce el mismo conjunto de archivos que sin la opción, y el manifiesto refleja la diferencia de estado.

**Decisiones pendientes.**
- ¿`--no-verify` debe seguir existiendo si no hay base de referencia (tras 1.1)? Si solo controla la validación de formato, el nombre debería ser `--no-format-check`.

---

### 1.3 Parser de CUE con comillas y espacios

**Problema.** `parse_cue` (`src/bin_parser/mod.rs:38-50`) separa la línea `FILE` con `split_whitespace()` y toma `parts[1]`, así que `FILE "Digimon World.bin" BINARY` produce `Digimon`. La CLI exige `--bin` explícito, pero la resolución de respaldo `cue_path.parent().join(bin_path)` y la selección de pares en la GUI dependen de esta ruta.

**Objetivo.** Analizar la sintaxis CUE real (comillas opcionales) y no romper la extracción por nombre de disco con espacios.

**Pasos.**
1. Reescribir el tokenizado de la línea `FILE`: si tras `FILE` empieza una comilla, tomar el contenido hasta la comilla de cierre; si no, tomar el siguiente token. Mantener el tipo de sector (`BINARY`/`MOTOROLA`) como token aparte.
2. No romper CUE sin comillas ni con espacios alrededor de los separadores.
3. Revisar la GUI (`src/gui/`) por usos de `parse_cue` y por selección manual de BIN, y unificar la resolución de la ruta del BIN (prioridad: `--bin` explícito > ruta relativa al CUE > selección en GUI).
4. Añadir un aviso en `docs/reference/CLI.md` cuando la ruta resuelta no exista.

**Comprobaciones de aceptación.**
- Pruebas unitarias de `parse_cue`: nombre con espacios y comillas, nombre sin comillas, CUE con varias líneas `FILE` (toma la primera), `TRACK ... MODE2/2352`, CUE sin `FILE` → error, sector no 2352 → error.
- Prueba de regresión: con el CUE real del juego (`Digimon World (Disc 1).cue`) la ruta resuelta es la del BIN real y `BinReader::open` funciona.

**Decisiones pendientes.**
- ¿Se acepta también la sintaxis `FILE "a.bin" BINARY 0` (offset) y `REM`? (compatibilidad con CUE de otros dumps).

---

## Fase 2 — Contrato de la CLI (Media)

### 2.1 `convert-tim`: recursión, salida y códigos de error

**Problema.** `run_convert_tim` (`src/cli/mod.rs:376-421`):
- El parámetro se llama `_recursive` y no se usa: el recorrido con `WalkDir` es siempre recursivo.
- Para un archivo individual entrega `--output` directamente a `TimHeader::to_png` (una ruta de archivo) aunque la ayuda lo define como directorio; además crea el directorio `output` antes (`create_dir_all`), lo que deja un archivo y un directorio con el mismo nombre.
- Los errores de conversión se imprimen y se absorben: el proceso termina con código 0 aunque todos los TIM fallen.

**Objetivo.** Un contrato único: `--output` es siempre un directorio, `--recursive` se respeta y un fallo produce código de salida distinto de 0.

**Pasos.**
1. Definir `--output` como directorio en todos los modos: archivo individual → `<output>/<nombre_del_tim>.png`; directorio → `<output>/<nombre>.png` (si dos archivos tienen el mismo nombre: sufijo inequívoco o error explícito).
2. Respetar `--recursive`: `WalkDir::new(input).max_depth(1)` cuando es `false` (solo el nivel superior).
3. Contar `converted`/`failed` y devolver error (o `exit(3)`) si `failed > 0`, con el detalle de los fallos por archivo.
4. Revisar la ayuda de `convert-tim` y `docs/reference/CLI.md` para que describan exactamente el contrato anterior.
5. Buscar el mismo patrón de «errores absorbidos» en el resto de `convert-*` y unificarlo (se aborda en el ciclo 2.2).

**Comprobaciones de aceptación.**
- Prueba de integración (`tests/convert_tim.rs`): directorio temporal con 2 TIM válidos en la raíz y 1 en un subdirectorio; con `--recursive` se convierten 3, sin `--recursive` solo 2.
- Prueba de integración: un TIM corrupto → el comando termina con código != 0 y el mensaje nombra el archivo.
- Prueba de integración: archivo individual → se crea `<output>/<stem>.png` (no un archivo llamado `--output`).
- `cargo test --all-targets` en verde.

**Decisiones pendientes.**
- ¿Debe `convert-tim` fallar si 0 de N conversiones tienen éxito, o basta con que 1 falle? (propuesta: si `failed > 0` → código != 0).
- ¿Se acepta `--output` como archivo cuando la entrada es un solo archivo (modo compatible)? Propuesta: no; se documenta el cambio como ruptura menor y se anota en `FEATURES.md`.

---

### 2.2 Propagación de errores en el resto de `convert-*`

**Problema.** `run_convert_audio` y las rutas MIDI/SF2 (`src/cli/mod.rs:445-620`) recorren directorios con `walkdir` e imprimen los errores por archivo, pero el resumen no influye en el código de salida.

**Pasos.**
1. Unificar el patrón: acumular `failed` y terminar con código != 0 si `failed > 0`, en todos los `convert-*`.
2. Extraer un helper común (`run_conversion_loop`) si la repetición lo justifica, sin reescribir los conversores.

**Comprobaciones de aceptación.**
- Prueba de integración por comando: entrada inválida → código != 0.

---

## Fase 3 — Pistas STR (Media)

### 3.1 Volcado de las pistas STR desde el BIN

**Problema.** `src/bin_parser/mod.rs` solo acepta sectores de datos: `extract_user_data` (`src/bin_parser/mod.rs:153`) rechaza con «Not a data sector (submode=0x…)» cualquier sector que no tenga el bit `0x08`, y devuelve 2048 bytes de datos de usuario. Las pistas `ED2.STR`, `EDR.STR`, `OP1.STR` y `OP2.STR` son sectores form 2 de vídeo y audio: hoy no se pueden extraer. `convert-str` (ya reescrito con el layout real: sync 12 + cabecera CD 4 + subheader 8 + datos de usuario 2048/2324) solo acepta el flujo de sectores crudos de 2352 bytes.

**Contexto verificado.** Con el BIN real: `OP1.STR` empieza en el sector 107652; los sectores de vídeo llevan submode `0x48` y los de audio `0x64` (AUDIO|FORM2|REAL_TIME), coding `0x01` (estéreo, 37800 Hz, 4 bits). Los sound groups empiezan en el offset 24. La extracción del ciclo 3.1 volcó 18 600 sectores (43 747 200 bytes); `convert-str` reconoció 3719 fotogramas y una pista XA. El WAV generado mide 17 859 870 bytes. La antigua referencia a 1500 sectores/300 fotogramas describe solo un fragmento manual y no el archivo completo.

**Decisión de formato (resuelta en 3.1).** La tabla de archivos (`data/file_table_us.json`) declara las pistas con 2336 bytes por sector, que no cuadra con el sector crudo de 2352 bytes. Se eligió (a), compatible con el lector actual:
- **(a) Archivo de 2352 bytes por sector** (sector crudo íntegro, incluye sync y EDC/ECC). Es lo que consume `convert-str` hoy. El tamaño del archivo (sectores × 2352) excede el tamaño declarado en la tabla.
- **(b) Archivo de 2336 bytes por sector** (sin los 12 bytes de sync; conserva la cabecera CD de 4 bytes, el subheader y la carga). Coincide con el tamaño de la tabla. El sync se puede reconstruir de forma determinista a partir del índice de sector (minuto/segundo/frame/modo), de modo que `convert-str` podría aceptar también este formato re-sincronizando.

**Objetivo.** Que las pistas STR se extraigan del BIN con el extractor normal y que su formato de archivo esté documentado.

**Pasos.**
1. En `src/bin_parser/mod.rs`, añadir `read_raw_sectors(start, count) -> Vec<u8>` (ya existe `read_sector`) y un modo de sector «sin restriction de submode».
2. En `src/extractor/mod.rs`, tratar las entradas `.STR` de la tabla de forma especial: escribir el flujo de sectores (2352 o 2336 según la decisión) en lugar de los datos de usuario.
3. Ajustar el tamaño que se registra en el manifiesto para esas entradas (el tamaño real del archivo volcado, no el de la tabla) y documentar la discrepancia.
4. En `src/formats/video/str.rs`, aceptar el formato de 2336 bytes por sector (reconstrucción del sync) si se elige la opción (b); si se elige la (a), documentar únicamente el flujo de 2352.
5. Actualizar `docs/reference/CLI.md` y `docs/reference/FORMATS.md` con el procedimiento de extracción y el formato resultante.

**Comprobaciones de aceptación.**
- **Hecho:** prueba unitaria del lector con sectores sintéticos de vídeo `0x48` y audio `0x64`; verifica preservación byte a byte y rechazo de rangos fuera del BIN.
- **Hecho:** extracción real de `OP1.STR`: primer sector empieza con sync `00 FF FF FF FF FF FF FF FF FF FF 00`, 18 600 sectores × 2352 = 43 747 200 bytes. `convert-str` reconoce 3719 fotogramas y una pista XA; produce WAV estéreo `audio_f1_ch1_stereo_37800hz_4bit.wav` de 17 859 870 bytes.
- **Pendiente para cierre completo:** comparar el WAV contra una extracción manual independiente; no hay golden manual completo disponible en el repositorio.
- **Hecho:** regresión real con `ALLTIM.TIM`: la salida mantiene los 3 317 760 bytes declarados y la validación TIM identifica 4 bpp, 256×128, CLUT presente. La ruta normal no cambió.

**Decisión tomada:** opción (a), sectores crudos completos de 2352 bytes, manteniendo el nombre `.STR` y jerarquía existente.

---

### 3.2 Honestidad de la ayuda de `convert-str`

**Problema.** `--extract-video` escribe `frame_*.mdec`: son bitstreams MDEC comprimidos, no vídeo reproducible. La redacción actual puede inducir a pensar lo contrario.

**Pasos.**
1. Ajustar la ayuda del subcomando y la salida por pantalla: «extracción de bitstream MDEC» y, por separado, «audio decodificado a WAV (XA-ADPCM)».
2. Documentar en `docs/reference/CLI.md` qué es y qué no es cada salida, y enlazar con la fase 6 (decodificador MDEC) como trabajo pendiente.
3. Anotar en `docs/quality/INTEGRITY_REVIEW.md` que el hallazgo sigue abierto hasta que exista la ruta de decodificación.

**Comprobaciones de aceptación.**
- `dw-extract convert-str --help` refleja el nuevo contrato.
- `docs/reference/CLI.md` actualizado.

---

## Fase 4 — Validación contra el juego real (Media)

### 4.1 SCN: contraste con los scripts del disco

**Problema.** `src/formats/script/` (parser, desensamblador y YAML) solo se ha probado con scripts sintéticos (`test_support::ScriptBuilder`). No se ha contrastado con un `DG.SCN` real ni con el intérprete de la referencia (`dw-engine/formats/src/script/scn_file.cpp`, `disassembly.cpp`).

**Objetivo.** Evidencia de que el desensamblado y el YAML son correctos sobre los scripts reales.

**Pasos.**
1. Extraer los `DG.SCN` del BIN real con `extract-files`.
2. Ejecutar `convert-scn --format disasm` y `--format yaml` sobre ellos.
3. Generar el golden de la referencia (opción A del entorno) o aplicar criterios de coherencia (opción B): terminador por tamaño de archivo, rango de `code_start`, coherencia de la tabla de offsets, texto decodificado legible con el charset de 752 códigos.
4. Versionar el golden como fixture en `tests/test_extract/` y añadir una prueba de regresión que compare la salida con él.

**Comprobaciones de aceptación.**
- Prueba de regresión: la salida de `convert-scn` sobre el fixture coincide byte a byte con el golden.
- Si no hay golden de la referencia: un informe en `docs/` con los criterios de coherencia evaluados y su resultado, y el hallazgo marcado como «validado estructuralmente, no contrastado».

**Decisiones pendientes.**
- ¿El YAML es un formato «del juego»? No: es propio, estilo referencia. Decidir si se etiqueta como salida propietaria en la documentación.

---

### 4.2 Audio: validar WAV/SF2/MIDI contra los bancos reales

**Problema.** `convert-audio` produce WAV (VAG→PCM), SF2 y MIDI desde los bancos del juego, validado solo con fixtures sintéticos.

**Pasos.**
1. Extraer los bancos reales (`SND/*.VAB`/`*.VHB`) del BIN.
2. Para VAG: contrastar el número de muestras y el bucle (`loop_info`) con la referencia `dw-engine/formats/src/audio/vag*`; comprobar que la longitud de cada muestra es múltiplo de 28 y que el tono coincide con la tabla de frequencies.
3. Para SF2: comprobar que lossamples quedan vinculados a sus números de muestra y que el SF2 resultante se puede abrir en un lector estándar.
4. Para MIDI: verificar que la secuencia extraída de la VAB es reproducible (mismo número de notas, mismos tiempos) y que el MIDI resultante es válido (SMF con los tracks esperados).
5. Versionar los fixtures de regresión.

**Comprobaciones de aceptación.**
- Pruebas de regresión con el banco real: hashes de las muestras decodificadas, nº de muestras por banco, tamaño del SF2/MIDI generado.
- Un WAV de la muestra 1 de un banco real suena sin clippings y con la duración esperada (frames = muestras / 44100 * 2).

---

## Fase 5 — Prototipos de mapa (Media)

### 5.1 Texturas MAP: extraer las secciones TIM

**Problema.** `export_map_textures` (`src/formats/map/map_yaml.rs:384`) solo crea el directorio `textures/` e imprime «Texture extraction requires original file data with TIM offsets»; no extrae nada. La referencia (`dw-engine/formats/src/map/map_file.cpp:208-217`) valida que cada sección de imagen empieza con un TIM de 8 o 4 bits (magic `0x10`) y deja la sección como rango de bytes para el lector de imágenes. El lector Rust (`src/formats/map/map_file.rs:130`) conserva `bits_per_pixel`, `offset` y `size`, pero descarta los bytes.

**Objetivo.** Extraer de verdad las texturas de los mapas con TIM.

**Pasos.**
1. Conservar los bytes de cada sección de imagen en el lector (o releerlos del archivo original por `offset`/`size`).
2. Validar el TIM (magic `0x10`, modo 4 u 8 bits) igual que la referencia y reportar las secciones inválidas.
3. Escribir un `textures/section_<i>_<4|8>bit.tim` por sección y convertirlo a PNG reutilizando `src/formats/tim.rs` (`to_png`).
4. Si `--extract-textures` se activa, registrar las texturas en el YAML y en el OBJ.

**Comprobaciones de aceptación.**
- Prueba de integración con un mapa real del disco: número de texturas extraídas = número de secciones TIM, cada PNG con dimensiones coherentes con el TIM (16×16 CLUT, 4 bpp o 8 bpp, o 32×32 4 bpp) y paleta no vacía.
- Prueba unitaria: sección con TIM corrupto → error con el índice de la sección, sin pánico.

### 5.2 Alcance del OBJ y ruta TFS en `convert-map`

**Problema.** El OBJ es un plano aproximado a partir de tiles/colisión y algunos puntos de entidades; no es la geometría del juego. `convert-map` no ofrece conversión TFS independiente aunque existe `src/formats/map/tfs_file.rs` (y la referencia `tfs_file.cpp`, con la paleta día/crepúsculo/noche).

**Pasos.**
1. Marcar la salida OBJ como aproximada en la ayuda de la CLI, en `docs/reference/CLI.md` y en `docs/reference/FORMATS.md` (mismo tratamiento que el resto de salidas parciales).
2. Añadir `--format tfs` a `convert-map` para volcar el TFS (celdas, atributos, texturas, paletas) de forma independiente, siguiendo la referencia.
3. Documentar qué partes del mapa quedan sin representar.

**Comprobaciones de aceptación.**
- Prueba de integración: `convert-map --format tfs` sobre un `.TFS` real produce un volcado con el número de celdas esperado según las dimensiones de la cabecera.
- La ayuda y la documentación etiquetan el OBJ como aproximado.

---

## Fase 6 — MDEC: de bitstream a imagen (Media, condicional)

**Problema.** No hay decodificador de vídeo: `convert-str --extract-video` entrega `frame_*.mdec`. La referencia tiene `dw-engine/formats/src/video/mdec.cpp` (26 834 B) y `mdec.hpp` (5 320 B), más `movie.cpp` (4 396 B) que hace la conversión de color y la escritura de fotogramas.

**Objetivo.** Decidir y ejecutar la ruta de vídeo visible, o cerrar el hallazgo como «solo bitstream» de forma explícita.

**Pasos (si se aprueba).**
1. Portar el decodificador MDEC: cabecera del bloque, codificación por bloques de 16×16, Run Length, DC/AC, cuantización y zigzag, tal como en `mdec.cpp`.
2. Convertir YUV a RGB con la matriz y el offset de PSX (implementados en `movie.cpp`).
3. Producir fotogramas PNG (o un AVI) a partir de los `frame_*.mdec` ya extraídos, con el ensamblaje de fotogramas de `src/formats/video/str.rs` como fuente de los datos.
4. Añadir `--extract-frames` (o `--video`/`--avi`) al subcomando y documentar el resultado.

**Comprobaciones de aceptación.**
- Fixture de un bloque MDEC conocido (golden de la referencia) → YUV esperado bit a bit.
- Conversión completa de una pista STR real: fotogramas decodificados sin errores, y un vídeo reproducible revisado a ojo (primeros fotogramas correctos, movimiento suave, colores sin bandas severas).

**Decisiones pendientes.**
- ¿Se quiere vídeo visible en este proyecto, o el bitstream basta para el uso previsto? Si no se aprueba, el ciclo se cierra documentando el alcance y el hallazgo H6 queda como «documentado, no implementado».

---

## Fase 7 — Reproducibilidad (Baja/Media)

### 7.1 Fijar las dependencias

**Problema.** `.gitignore` excluye `Cargo.lock`, así que las versiones resueltas pueden cambiar entre builds. Para un binario, Rust recomienda versionar el lockfile.

**Pasos.**
1. Decidir: se versiona (recomendado) o se mantiene excluido de forma deliberada.
2. Si se versiona: quitar la regla de `.gitignore`, regenerar el lockfile y revisar que no haya versiones dudosas.
3. Si no: documentarlo en `README.md`/`FEATURES.md` y añadir una nota de que los builds no son bit a bit reproducibles.

**Comprobaciones de aceptación.**
- Si se versiona: `cargo build --locked` funciona en una copia limpia del repositorio.
- Revisión de seguridad del lockfile: sin dependencias duplicadas evitables ni crates abandonados sin motivo.

---

## Estrategia de pruebas

Para cada ciclo:

1. **Unitarias** — caso feliz, casos límite y entrada malformada/truncada.
2. **Integración** (`tests/`) — comando completo sobre un fixture temporal o un archivo del juego; comprobación del código de salida y del contenido generado.
3. **Datos reales** — con el BIN (`base/Digimon World Vice.bin`) o con archivos extraídos de él; cuando el BIN no esté disponible, el test se marca como manual y el procedimiento queda escrito.
4. **Regresión contra referencia** — golden de `dw-engine` cuando exista; si no, criterios de coherencia documentados.
5. **Suite completa** — `cargo test --all-targets` en verde y sin warnings nuevos introducidos por el ciclo.

**Comandos de verificación habituales**

```powershell
cargo test --all-targets          # suite completa
cargo build --release             # build de release
.\target\release\dw-extract.exe --help
.\target\release\dw-extract.exe convert-str --help
```

---

## Criterios de éxito

- [ ] H1 cerrado: no queda ninguna referencia con hash inventado, y la verificación (si existe) tiene fuente comprobable.
- [ ] H2 cerrado: `--no-verify` se propaga en `extract` y `extract-files`, con prueba.
- [ ] H3 cerrado: `convert-tim` respeta `--recursive`, usa `--output` como directorio y devuelve código != 0 ante fallos.
- [ ] H4 cerrado: `parse_cue` respeta comillas y espacios, con pruebas.
- [ ] H5 cerrado: las texturas MAP se extraen de verdad o la opción se retira; el OBJ y el TFS están documentados con su alcance real.
- [ ] H6 cerrado: la ayuda y la documentación dicen exactamente qué entrega `convert-str`, y el decodificador MDEC está implementado o el alcance queda asumido por escrito.
- [ ] H7 cerrado: las pistas STR se extraen del BIN con el extractor normal y su formato de archivo está documentado.
- [ ] H8 cerrado: `convert-scn` está contrastado con los scripts reales (golden o criterios documentados).
- [ ] H9 cerrado: política de `Cargo.lock` decidida y aplicada.
- [ ] H10 cerrado: `convert-audio` validado contra los bancos reales.
- [ ] `FEATURES.md` y `docs/quality/INTEGRITY_REVIEW.md` actualizados con el estado real de cada punto.
- [ ] `cargo test --all-targets` en verde (baseline actual: 103 pruebas).

---

## Fuera de alcance

- Añadir formatos nuevos (eso vive en [FORMAT_SUPPORT_PLAN.md](FORMAT_SUPPORT_PLAN.md)).
- Decodificadores de MDEC más allá del alcance acordado en la fase 6.
- Mux a MP4 y compresión de vídeo.
- Empaquetado/verificación del disco completo de extremo a extremo.
- Cualquier cambio en las referencias de la tabla de archivos (`data/file_table_us.json`) sin una fuente comprobable.

---

## Preguntas abiertas (para decidir antes de empezar)

1. ¿Se dispone de una lista Redump real (o de un golden de `dw-engine` generado en otra máquina) para cerrar H1, H8 y H10 con evidencia, o se trabaja solo con criterios de coherencia?
2. ¿Se quiere vídeo visible (fase 6) o el bitstream MDEC es suficiente? De la respuesta depende si H6 se implementa o se documenta el alcance.
3. Formato de archivo para las pistas STR: 2352 bytes por sector (sector crudo) u 2336 (alineado con la tabla de archivos)?
4. ¿La CLI sigue admitiendo `--output` como archivo en `convert-tim` por compatibilidad, o se cambia a directorio sin excepciones?
5. ¿Se versiona `Cargo.lock`?

---

## Registro de avance

| Ciclo | Estado | Fecha | Evidencia |
|---|---|---|---|
| 1.1 | Pendiente | | |
| 1.2 | Pendiente | | |
| 1.3 | Pendiente | | |
| 2.1 | Pendiente | | |
| 2.2 | Pendiente | | |
| 3.1 | Implementación y E2E parcial | 2026-10-21 | Prueba Form 2; OP1.STR extraído y convertido (3719 frames, WAV 17 859 870 B); ALLTIM.TIM normal mantiene 3 317 760 B. Falta golden WAV independiente. Ver [FORMATOS](../reference/FORMATS.md#str--ciclo-31-volcado-desde-bin). |
| 3.2 | Pendiente | | |
| 4.1 | Pendiente | | |
| 4.2 | Pendiente | | |
| 5.1 | Pendiente | | |
| 5.2 | Pendiente | | |
| 6.1 | Pendiente (condicional) | | |
| 7.1 | Pendiente | | |

---

*Creado: 2026-10-21 · basado en `docs/quality/INTEGRITY_REVIEW.md` (2026-10-21) y `FEATURES.md`*
