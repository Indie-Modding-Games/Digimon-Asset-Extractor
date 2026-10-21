# Estado de formatos

Este documento separa tres cosas que el código no debe confundir: **extracción cruda desde el disco**, **código de biblioteca presente** y **conversión disponible por CLI**. La presencia de un módulo no significa que el formato se haya validado con todos los archivos del juego.

| Formato/familia | Código Rust observado | CLI actual | Alcance comprobable y límites |
|---|---|---|---|
| BIN/CUE, sectores PS1 | `bin_parser/` | `extract`, `extract-files` | Lee MODE2/2352 y 2048 bytes de usuario Form 1; tabla fija para SLUS-01032. EDC se advierte, no detiene la extracción. |
| TIM | `formats/tim.rs` | `convert-tim` | Lee metadatos y decodifica 4/8/16/24 bpp a PNG. El conversor CLI tiene problemas de ruta de salida y recorrido; ver [revisión](../quality/INTEGRITY_REVIEW.md). |
| MMD/TMD | `formats/mmd.rs` | Validación durante extracción | Comprueba encabezado/objetos y busca un TIM embebido. No exporta geometría OBJ/GLTF. |
| VAB/VHB/VAG | `formats/vab.rs`, `vab2.rs`, `vag.rs` | `convert-audio` | El lector VAB/VHB sigue el layout de la referencia: cabecera `pBAV` de 32 bytes, bloques de tonos por programa con tonos y tabla de tamaños en octavos de byte. Conversión a WAV, raw, SF2 y MIDI. La cobertura de bancos y muestras reales no está demostrada por la batería actual. |
| PSX ADPCM / XA-ADPCM | `formats/adpcm.rs`, `formats/video/xa_adpcm.rs`, `wav.rs` | Audio y STR | El decodificador VAG replica el de la referencia: residual con signo extendido desplazado, predicción del filtro con redondeo, valores reservados (shift > 12, filtro > 4) y detección de bucles por banderas. El decodificador XA-ADPCM también replica el de la referencia: 18 grupos de sonido por sector, 8 unidades de 28 muestras, filtros `{0,0},{60,0},{115,-52},{98,-55}` en 64avos, shift reservado 13–15 → 9, saturación a 16 bits, historia por canal entre grupos y sectores, intercalado estéreo y conteo de parámetros que difieren de su copia; validado con los 8 casos de prueba de la referencia. |
| SEQ/MIDI/SF2 | `formats/audio/` | `convert-audio --format midi\|sf2` | Hay parser de SEQ, escritor MIDI (formato 0, tempo, RPN de bend range, marcadores de bucle), escritor SF2 (PHDR, PBAG, PMOD, PGEN, INST, IBAG, IMOD, IGEN, SHDR) y conversión VAB→SF2 con el envelope ADSR aproximado. No se ha contrastado con reproductores externos ni con el corpus completo del juego. |
| EFE | `formats/effect/` | `convert-efe` | Lee slots, decodifica instrucciones/disassembly y escribe YAML/JSON; puede extraer bloques TMD/TIM. No es un intérprete VM ni un visor/renderizador de efectos. |
| MAP | `formats/map/map_file.rs`, `map_yaml.rs` | `convert-map` | El parser actual no coincide con el layout de referencia y puede producir una lectura parcial que termina como éxito. No usar su YAML/OBJ como exportación fiel hasta corregirlo. |
| TFS | `formats/map/tfs_file.rs` | Ninguno | El layout actual de Rust no coincide con la referencia C++; el lector no es seguro para archivos reales y no debe usarse hasta reemplazarlo y validarlo. |
| STR | `formats/video/str.rs` | `convert-str` | Lee el flujo de sectores crudos (sync 12 + cabecera 4 + subheader 8 + datos de usuario), valida el StHEADER de vídeo, ensambla fotogramas comprobando `headm`/`headv` y el relleno, y decodifica el audio XA a WAV con el estado por pista. La opción de vídeo guarda bitstreams `.mdec`; no decodifica a PNG/MP4. |
| SCN | `formats/script/` | `convert-scn` | Lee tabla de scripts, desensambla, decodifica texto y exporta YAML o listado; validado con fixtures sintéticos, falta contraste con scripts reales. |
| Fuentes/charset | `formats/script/text.rs` | `convert-scn` | La tabla de 752 códigos del charset está implementada y verificada contra la referencia; no hay atlas PNG de fuentes ni exportador BMFont/JSON. |
| EXE/overlay | Ninguno observado | Ninguno | Planeado; no implementado en el árbol actual. |

## Evidencia y confianza

### STR — ciclo 3.1 (volcado desde BIN)

La extracción normal detecta ahora los archivos `.STR` por extensión y vuelca sus sectores crudos completos (2352 bytes), sin filtrar submode. La tabla USA expresa el tamaño de estas entradas como 2336 bytes por sector; ese valor se usa para calcular el número de sectores, mientras que el manifiesto registra el tamaño real escrito (`sectores × 2352`). Los demás formatos conservan la extracción habitual de 2048 bytes de datos de usuario por sector.

Este formato conserva sync, cabecera, subheader y carga y es directamente compatible con `convert-str`. No se reconstruye ningún byte del sector. La validación con el BIN real y la comparación del WAV/fotogramas con el volcado manual siguen pendientes de ejecución para cerrar la aceptación de extremo a extremo.

### MAP/TFS — ciclo 3.1 (estructura contrastada)

La referencia consultada es `dw-engine/formats/include/dwe/formats/map/{map_file,tfs_file}.hpp` y `dw-engine/formats/src/map/{map_file,tfs_file}.cpp`. Se extrajeron `MAYO01.MAP` y `MAYO01.TFS` del BIN de prueba del proyecto a `test_extract/map_samples/` (directorio local ignorado por Git).

**Layout MAP según la referencia:** el primer `u32` es el tamaño en bytes de la tabla de offsets; dividirlo entre cuatro da el número de secciones. Hay tres secciones fijas (setup, entities, collision); cada mapa con imágenes incluye secciones TIM individuales, ordenadas 8 bpp antes de 4 bpp, y una sección de objetos. Cada offset delimita una sección hasta el offset siguiente; la última llega al fin del archivo. El setup contiene cámara, tres luces, ambiente/proyección, dos grupos de cuatro valores de región, dimensiones y una cuadrícula de `width × height` enteros. Entities contiene seis arrays de 10 warps y el conteo Digimon; la presencia de registros NPC depende de `MAP_ENTRIES.flags`, no de un indicador inequívoco dentro de MAP. Colisión es una cuadrícula fija de 100 × 100 bytes.

**Layout TFS según la referencia:** `u32 unknown00`, `u32 paletteCount` (máximo 31), paletas de 256 colores `u16`, y luego cero o más tiles. Cada tile contiene un `u32 unknown00` más 128 × 128 índices de píxel de 8 bits. En los archivos USA, el primer desconocido codifica dimensiones en píxeles; el desconocido de cada tile codifica una posición de cuadrícula. La función de paletas posteriores a día/atardecer/noche sigue sin conocerse.

**Muestra real `MAYO01`:** MAP de 177,964 bytes; primer offset `28` (siete offsets), secciones que comienzan en `28, 300, 66,380, 132,460, 165,292, 167,002, 167,964`. Tiene dos TIM de modo 8 bpp y uno de 4 bpp. Su setup mide 6 × 5. TFS mide 493,184 bytes, comienza `0x00050006, 3`, y contiene 3 paletas y 30 tiles completos. El primer word corresponde a 6 × 5 según la codificación descrita por la referencia.

**Discrepancias Rust demostradas:** `read_map_file_shape` y `read_map_file` leen exactamente seis offsets fijos; para la muestra ignoran el séptimo offset y las secciones no quedan alineadas con la referencia. `MapImageSection::read` espera un descriptor de 12 bytes donde el archivo contiene un TIM. La CLI `convert-map` se ejecutó con esa muestra, terminó en éxito, pero reportó cero imágenes y cero sprites; esto es una lectura parcial silenciosa. `infer_map_read_options` supone NPC presentes cuando el tamaño de entities supera el mínimo; la referencia considera esa inferencia ambigua y devuelve error salvo cuando el tamaño prueba que no hay registros NPC.

`formats/map/tfs_file.rs` espera cinco enteros (`version`, `width`, `height`, `cell_size`, `flags`) y celdas fijas de 16 bytes. Es incompatible con el layout de paletas/tiles de la referencia. No se ejecutó sobre el TFS real: con dimensiones interpretadas como datos de paleta podría reservar una cantidad peligrosa de memoria.

**Campos todavía desconocidos:** en registros NPC, offsets `0x12` (`unknown12`) y `0x2A` (`unknown2A`); en TFS, el propósito de paletas después de las tres variantes conocidas. Los dos metadatos `u32` se usan como codificación en USA, pero la semántica general no está establecida.

- La tabla BIN indica 818 entradas, incluyendo 178 MMD y 52 TIM. Es un conteo de metadatos, no una prueba de que todos esos archivos se hayan extraído/validado en este entorno.
- El repositorio contiene muestras en `tests/test_extract/` (dos MMD y `CHDAT/ALLTIM.TIM`) y `tests/test_tim/OP.TIM`.
- `cargo test --all-targets` pasa 103 pruebas (98 de biblioteca más 5 de integración end-to-end de la CLI). La mayoría de pruebas de formato usa fixtures sintéticos; la ejecución no equivale a una campaña de validación sobre el disco completo.
- El audio XA y el lector de sectores STR se comprobaron además contra el BIN real (`base/Digimon World Vice.bin`, pista OP1.STR): 2000 sectores → 400 fotogramas y una pista estéreo 37800 Hz/4 bits, con el WAV stereo válido (2 016 000 bytes de muestras, pico ±24414, RMS 3588).
- El estado real de salida/CLI se confirma en [CLI](CLI.md); los trabajos futuros están en el [plan](../planning/FORMAT_SUPPORT_PLAN.md).
