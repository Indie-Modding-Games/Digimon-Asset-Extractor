# Format Support Implementation Plan

**Project:** Digimon World Asset Extractor (dw-extract)  
**Reference Source:** `C:\Users\Sebastian\Desktop\Projects\dw-engine\formats`  
**Target Location:** `src/formats/` (Rust)  
**Status:** 10/12 formats implemented; 2 remaining

---

## Current Implementation Status (as of 2026-10-21)

### ✅ Implemented Formats (10/12)
| Format | Source Files | CLI Commands |
|--------|--------------|--------------|
| **TIM** | `tim.rs` | `convert-tim` |
| **MMD/TMD** | `mmd.rs` | `extract`, `extract-files` |
| **VAB/VHB** | `vab.rs`, `vab2.rs` | `convert-audio` |
| **VAG/ADPCM** | `vag.rs`, `vab.rs`, `adpcm.rs`, `wav.rs` | `convert-audio` |
| **EFE** | `effect/efe_file.rs`, `efe_script.rs`, `efe_yaml.rs` | `convert-efe` |
| **MAP/TFS** | `map/map_file.rs`, `map/tfs_file.rs`, `map/map_yaml.rs` | `convert-map` |
| **STR Video** | `video/str.rs`, `video/xa_adpcm.rs` | `convert-str` |
| **SCN Scripts** | `script/scn_file.rs`, `script/disassembly.rs`, `script/script_yaml.rs`, `script/text.rs` | `convert-scn` |
| **SEQ/MIDI** | `audio/seq.rs`, `audio/midi.rs` | `convert-audio --format midi` |
| **SF2** | `audio/sf2.rs`, `audio/vab_sf2.rs`, `audio/envelope.rs` | `convert-audio --format sf2` |

### 📋 Planned Formats (2/12)
| Format | Priority | dw-engine Source |
|--------|----------|------------------|
| **Font/Charset** | 🟢 Low | `formats/text/charset.{hpp,cpp}`, `font.{hpp,cpp}` |
| **Executable/Overlay** | 🟢 Low | `formats/exe/*.{hpp,cpp}` |

### CLI Commands Available
- `extract` - Extract all files from BIN/CUE
- `extract-files` - Extract specific files by name
- `convert-tim` - Convert TIM files to PNG
- `convert-audio` - Convert VHB/VAB audio files to WAV, raw, SF2 or MIDI
- `convert-efe` - Convert EFE effect files to YAML
- `convert-map` - Convert MAP/TFS map files to YAML/OBJ
- `convert-scn` - Convert SCN scripts to YAML/disassembly
- `convert-str` - Convert STR video files to frames and audio
- `list` - List all files in the file table
- `verify` - Verify existing extracted assets against manifest
- `info` - Show file table info

---

## Remaining Implementation Plan

### Phase 1: Audio Extensions - SEQ/MIDI/SF2 (Priority: 🟡 Medium) — ✅ Completado

**Target Output:** MIDI files, SoundFont 2 (SF2) banks, VAB→SF2 conversion

**dw-engine Source:**
- `formats/include/dwe/formats/audio/seq.hpp` (2,663 lines) - SEQ format
- `formats/src/audio/seq.cpp` (9,233 lines) - SEQ parser
- `formats/include/dwe/formats/audio/midi.hpp` (1,398 lines) - MIDI types
- `formats/src/audio/midi.cpp` (6,054 lines) - MIDI writer
- `formats/include/dwe/formats/audio/sf2.hpp` (2,707 lines) - SF2 structures
- `formats/src/audio/sf2.cpp` (9,521 lines) - SF2 writer
- `formats/include/dwe/formats/audio/vab_sf2.hpp` (1,946 lines) - VAB→SF2 conversion
- `formats/src/audio/vab_sf2.cpp` (5,650 lines) - Full conversion pipeline
- `formats/src/audio/envelope.cpp` (5,985 lines) - ADSR envelope approximation

**Cycles (completados):**
1. **1.1** Estructuras de SEQ: cabecera `pQES` (resolución, tempo, compás), eventos con estado en curso, finales y marcadores de bucle; layout del banco de secuencias (la primera entrada de la tabla es su propio tamaño).
2. **1.2** Parser de SEQ con validación de flujos malformados y tests portados de `seq_tests.cpp`.
3. **1.3** Escritor MIDI: formato 0, división igual a la resolución, tempo, RPN de bend range por programa, marcadores `loopStart`/`loopEnd` y nota-on con velocidad 0 conservada; test golden.
4. **1.4** Escritor SF2 con el layout de la referencia: `RIFF`/`sfbk`, listas `INFO`/`sdta`/`pdta`, tablas `phdr`, `pbag`, `pmod`, `pgen`, `inst`, `ibag`, `imod`, `igen`, `shdr`, registros terminales de 46 bytes, chunks de tamaño par y nombres con NUL reservado.
5. **1.5** Conversión VAB→SF2: un preset por programa, una zona por tono, una muestra por VAG tocado, generadores (atenuación, pan, reverb, envelope, afinación, nota raíz, modo de bucle) y el envelope ADSR del chip aproximado paso a paso a 44100 Hz, con caché por registro.
6. **1.6** CLI: `convert-audio --format midi|sf2`. Un music font produce `track_NN.mid` por pista; una secuencia suelta produce `<nombre>.mid`; con varios archivos, cada uno recibe su propio subdirectorio. `sf2` escribe `output.sf2`.

**Trabajo de fidelidad adicional realizado en la fase:**
- Lector VAB/VHB reescrito según la referencia: magic `pBAV`, offsets reales de la cabecera, bloques de tonos ligados al programa con tonos (no al número de slot), validación de `ps`, `fsize`, `vs` y de cada tono, y tabla de tamaños en octavos de byte.
- Decodificador VAG ADPCM portada de la referencia: residual con signo extendido (`nibble << 12 >> shift`), predicción `(s1 * k0 + s2 * k1 + 32) >> 6`, saturación a 16 bits, valores reservados (shift > 12 → 9, filtro > 4 → 4, bit 7 de la cabecera) y banderas `END`/`REPEAT`/`LOOP_START` con detección del bucle. El decodificador anterior no extendía el signo y restaba la predicción, por lo que las muestras salían incorrectas.
- `SampleLoop` pasa a ser el tipo compartido `adpcm::AdpcmLoop`; la CLI usa el bucle real de la muestra en vez de uno sintético.
- Decodificador XA-ADPCM portado de la referencia (`formats/video/xa_adpcm.rs`): 18 grupos de sonido por sector, 8 unidades de 28 muestras, byte de codificación con bits reservados y énfasis rechazados, filtros `{0,0},{60,0},{115,-52},{98,-55}` en 64avos, shift reservado 13–15 → 9, saturación a 16 bits, historia por canal entre grupos y sectores, intercalado estéreo y conteo de los parámetros que difieren de su copia. Los 8 casos de `xa_adpcm_tests.cpp` están portados.
- Lector de sectores STR reescrito según la referencia: sector crudo de 2352 bytes (sync 12 + cabecera 4 + subheader 8 + datos de usuario), datos de usuario de 2048 bytes para vídeo y 2324 para audio, StHEADER con marcador `0x0160`/`0x8001` y campo reservado a 0, relleno del fotograma a cero y `headm`/`headv` contrastados con el bitstream. El lector anterior leía el subheader 4 bytes antes de lo debido y no consumía el sector completo, de modo que el audio y los fotogramas salían corruptos.
- El audio STR se decodifica con un `XaDecoder` por pista (la historia cruza sectores) y se escribe en un WAV estéreo con el tamaño de datos correcto.

**Estado:** implementado con pruebas unitarias sobre fixtures sintéticos (`src/formats/audio/fixtures.rs`) y pruebas end-to-end de la CLI (`tests/convert_audio_midi.rs`, `tests/convert_audio_sf2.rs`); falta la validación contra los bancos de sonido reales del disco. El audio XA y el lector STR sí se comprobaron con la pista OP1.STR del BIN real: 2000 sectores → 400 fotogramas y una pista estéreo 37800 Hz/4 bits con un WAV válido.

**Nota sobre la GUI:** la GUI actual solo ofrece extracción (arrastre de BIN/CUE, progreso, log y configuración de salida); no tiene panel de audio, así que no hubo nada que actualizar. Añadir un panel de audio en la GUI sería una función nueva, no una actualización, y queda fuera de esta fase.

---

### Phase 2: SCN Scripts (Priority: 🟢 Low) — ✅ Completado

**Target Output:** Disassembly (text), YAML structure

**dw-engine Source:**
- `formats/include/dwe/formats/script/scn_file.hpp` (2,036 lines) - SCN structure
- `formats/src/script/scn_file.cpp` (2,300 lines) - SCN parser
- `formats/include/dwe/formats/script/disassembly.hpp` (4,995 lines) - Disassembler
- `formats/src/script/disassembly.cpp` (26,562 lines) - Full disassembly
- `formats/include/dwe/formats/script/script_yaml.hpp` (824 lines) - YAML
- `formats/src/script/script_yaml.cpp` (7,182 lines) - YAML export

**Cycles (completados):**
1. **2.1** Estructura SCN: tabla de offsets `0x2000`, rangos de script y terminador por tamaño de archivo.
2. **2.2** Header de script: `code_start`, pares `{id, offset}` terminados en `0xFFFF` y validación de coherencia.
3. **2.3** Disassembler: 105 opcodes despachados, operandos, flujo (`Next/Goto/Call/Stop/Switch/Branch`) y cuenta de código inalcanzable.
4. **2.4** Texto de scripts: charset de 752 códigos (verificado contra `charset.cpp`) y decodificación de celdas cp932.
5. **2.5** YAML: emitter propio que reproduce el estilo de la referencia (secuencias sin sangría, texto entre comillas).
6. **2.6** CLI: `convert-scn --input <scn> --output <dir> --format yaml|disasm`, con detección de contenedor o script individual.

**Estado:** implementado con pruebas unitarias sobre scripts sintéticos (`script/test_support.rs`); falta la validación contra los scripts reales del disco.

---

### Phase 3: Font/Charset (Priority: 🟢 Low)

**Target Output:** PNG font atlas, BMFont/JSON metadata

**dw-engine Source:**
- `formats/include/dwe/formats/text/charset.hpp` (2,090 lines) - Charset tables
- `formats/src/text/charset.cpp` (12,448 lines) - Charset parser
- `formats/include/dwe/formats/text/font.hpp` (3,486 lines) - Font structures
- `formats/src/text/font.cpp` (7,745 lines) - Font renderer
- `formats/include/dwe/formats/text/game_text.hpp` (3,189 lines) - Game text
- `formats/src/text/game_text.cpp` (19,249 lines) - Text extraction

**Proposed Cycles:**
1. **3.1** Inspect charset/font format from reference
2. **3.2** Implement charset parser and font atlas generation
3. **3.3** Export PNG + BMFont/JSON metadata
4. **3.4** CLI for font extraction

---

### Phase 4: Executable/Overlay (Priority: 🟢 Low)

**Target Output:** Function list, symbol table, overlay map

**dw-engine Source:**
- `formats/include/dwe/formats/exe/*.hpp` (6 files)
- `formats/src/exe/*.cpp` (6 files, ~80K lines total)
- Covers PSX EXE, overlays, constant tables, item effects

**Proposed Cycles:**
1. **4.1** Inspect PSX EXE and overlay format
2. **4.2** Implement parser for executable headers and overlay tables
3. **4.3** Export symbol table and function boundaries

---

## Implementation Guidelines

### Execution Protocol (Required)
1. Work in **one cycle at a time**. A cycle is a small change that can be reviewed and verified on its own.
2. At the start of each cycle:
   - Read relevant Rust modules, CLI/GUI integration points, `Cargo.toml`, and available tests
   - Check that the listed `dw-engine` reference path and files exist
   - State the cycle's goal, exact files/components expected to change, acceptance checks, and any unresolved choice
3. Implement only that cycle. Update this plan's status only when acceptance checks have evidence.
4. At the end of **every cycle**, run the cycle's specified checks, summarize the change, files, results, and remaining work, then ask the user explicitly whether to proceed to the next cycle, revise the result, or change priorities.
5. Each implementation cycle must include a focused automated test or explain why a meaningful automated test is not yet possible.

### Code Reuse Strategy
- **Behavioral Port** (not line-by-line translation): Identify only structures, enums, constants, and parsing rules needed for agreed output
- **Adapted Port** (Rust Idioms): `std::span<const std::byte>` → `&[u8]`, `core::Result<T>` → `Result<T, Error>`, `std::optional` → `Option`, manual memory → `Vec`, `Box`
- Use `nom` or custom byte cursor for parsing

### External Crates (Evaluate Before Adding)
| C++ Code | Rust Crate |
|----------|------------|
| PNG writer | `png` crate |
| GLTF writer | Evaluate suitable GLTF writing crate |
| WAV writer | `hound` or `wav` crate (already used) |
| YAML | `serde_yaml` (already used); an own emitter when the output must match the reference exactly, as in SCN |

---

## Testing Strategy
For each new format:
1. **Unit tests** - Test valid boundaries and malformed/truncated input
2. **Round-trip tests** - Parse → serialize → parse (where applicable)
3. **Real data tests** - Use actual game files from `tests/test_extract/`
4. **Regression/reference tests** - Compare with `dw-engine` when runnable reference available

---

## Dependencies to Evaluate
Inspect `Cargo.toml` first and add only what the approved cycle needs:
- `hound` = "3.5" (WAV writing - already present)
- `png` = "0.17" (PNG encoding)
- `gltf` = "0.5" (GLTF export - evaluate)
- `serde_json` = "1.0" (JSON export - already present)
- `rayon` = "1.8" (Parallel processing - evaluate)

---

## Success Criteria
- [ ] Each claimed format parses documented valid sample corpus and rejects malformed inputs
- [ ] Round-trip: extract → convert → verify matches original
- [ ] CLI commands work for all formats
- [ ] Relevant Rust tests pass; applicable upstream reference cases documented

---

## Appendix: Key dw-engine Files for Remaining Formats

```
Audio (SEQ/MIDI/SF2):
  formats/include/dwe/formats/audio/seq.hpp
  formats/include/dwe/formats/audio/midi.hpp
  formats/include/dwe/formats/audio/sf2.hpp
  formats/include/dwe/formats/audio/vab_sf2.hpp
  formats/src/audio/seq.cpp
  formats/src/audio/midi.cpp
  formats/src/audio/sf2.cpp
  formats/src/audio/vab_sf2.cpp

Script (SCN):
  formats/include/dwe/formats/script/scn_file.hpp
  formats/include/dwe/formats/script/disassembly.hpp
  formats/include/dwe/formats/script/script_yaml.hpp
  formats/src/script/scn_file.cpp
  formats/src/script/disassembly.cpp
  formats/src/script/script_yaml.cpp

Text (Font/Charset):
  formats/include/dwe/formats/text/charset.hpp
  formats/include/dwe/formats/text/font.hpp
  formats/include/dwe/formats/text/game_text.hpp
  formats/src/text/charset.cpp
  formats/src/text/font.cpp
  formats/src/text/game_text.cpp

Executable:
  formats/include/dwe/formats/exe/*.hpp
  formats/src/exe/*.cpp
```

---

*Updated: 2026-10-21*  
*Based on current implementation status (10/12 formats complete)*
