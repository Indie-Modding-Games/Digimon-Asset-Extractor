# Arquitectura

## Resumen

`dw-extract` es un paquete Rust con una biblioteca reutilizable y tres binarios declarados en `Cargo.toml`:

- `dw-extract`: CLI principal (`src/main.rs`).
- `dw-extract-gui`: interfaz eframe/egui (`src/gui_main.rs`).
- `parse-table`: herramienta de depuración (`debug/parse_table.rs`).

El proyecto está centrado en una tabla fija para la edición estadounidense SLUS-01032. No carga una tabla de archivos alternativa en tiempo de ejecución.

## Flujo de extracción de disco

```mermaid
flowchart LR
    A[Archivo CUE] --> B[Resolver archivo BIN y modo de pista]
    B --> C[BinReader: sectores MODE2/2352]
    C --> D[Tabla fija data/file_table_us.json]
    D --> E[Comprobar SYSTEM.CNF]
    E --> F[Leer 2048 bytes de usuario por sector Form 1]
    F --> G[Validar TIM/MMD cuando está activado]
    G --> H[Elegir ruta plana, YAML o CHDAT]
    H --> I[Escribir archivos y calcular hashes]
    I --> J[Manifiesto JSON opcional]
```

### BIN/CUE y sectores

- `src/bin_parser/` lee la línea `FILE ... BINARY` y el primer `TRACK ... MODE...` que reconoce. Requiere tamaño de sector 2352.
- El parser divide la línea `FILE` por espacios; una ruta BIN entrecomillada con espacios no se interpreta correctamente. Aunque CLI recibe `--bin`, el nombre del CUE se usa como respaldo si esa ruta no existe y la GUI también deriva un archivo del CUE.
- `BinReader` comprueba que el BIN tenga un tamaño múltiplo de 2352 y busca el patrón de sincronización, modo 2 y bit de sector de datos.
- Se extraen 2048 bytes de usuario por sector, como datos Form 1. EDC incorrecto genera una advertencia y no detiene la lectura; no hay una garantía de reparación/corrección ECC.
- El parser CUE es intencionalmente simple; no debe suponerse compatibilidad con CUE complejos, múltiples BIN o cualquier disposición de pistas.

### Tabla de archivos y versión

- `src/file_table/` incluye en compilación `data/file_table_us.json` y resuelve nombre/ruta, sector y tamaño.
- La extracción comprueba `SYSTEM.CNF` para reconocer SLUS-01032. Las versiones SLES/SLPS se rechazan.
- El inventario observado por `dw-extract info`: 818 archivos, 178 MMD y 52 TIM.
- La tabla deriva de `dw_decomp`; cambiar de región requiere otra tabla revisada y lógica de identificación propia.

### Escritura y organización

- `src/extractor/` coordina lectura por archivo, validación, hashes, protección contra sobrescritura y resumen.
- `src/config/` aplica reglas por extensión y por nombre desde YAML. El YAML predeterminado se incrusta en la compilación; se puede proporcionar otro mediante CLI/GUI.
- La opción CHDAT usa un mapeo especial adicional para los MMD conocidos y deja `ALLTIM.TIM` en la raíz de salida.
- `src/verification/` calcula MD5/SHA256, crea manifiestos y compara archivos extraídos con el manifiesto.

## Interfaces

### CLI

`src/cli/` define argumentos y orquesta extractores/conversores. La lógica binaria reside en biblioteca. Los subcomandos activos se enumeran en [CLI](../reference/CLI.md). No hay comandos para todas las fuentes de formato presentes.

### GUI

`src/gui/` implementa la interfaz eframe/egui. La GUI selecciona BIN/CUE, configura extracción, muestra avance y registros, y ejecuta la extracción en un hilo de trabajo. La GUI no expone los conversores EFE/MAP/STR/audio como paneles. Su función actual es principalmente la extracción del disco.

## Árbol de módulos principal

```text
src/
├── bin_parser/       CUE, sectores BIN y datos de usuario
├── file_table/       Tabla estadounidense integrada
├── extractor/        Extracción, salida y validación
├── verification/     Hashes, manifiesto y comparación
├── config/           Mapeo YAML de salida
├── cli/              Comandos y orquestación
├── gui/              Interfaz y extracción en segundo plano
└── formats/          Lectores y exportadores de formatos del juego
    ├── audio/        SEQ, MIDI, SF2 y conversión VAB→SF2
    ├── effect/       Contenedor EFE, instrucciones y YAML/JSON
    ├── map/          MAP/TFS, YAML y OBJ aproximado
    ├── script/       Contenedor SCN, disassembler, texto de scripts y YAML
    └── video/        Sectores STR, fotogramas MDEC comprimidos y audio XA
```

El módulo `src/formats/script/` está declarado desde `src/formats/mod.rs` y expone `parse_scn_file`, `disassemble_script`, `decode_script_text` y `script_to_yaml`.

## Compilación

El paquete declara varios binarios, por lo que especifica el binario al usar Cargo:

```powershell
cargo build --release --bin dw-extract
cargo build --release --bin dw-extract-gui
cargo run --bin dw-extract -- --help
cargo run --bin dw-extract-gui
```

Las dependencias abarcan CLI, GUI, serialización, imágenes, WAV y parseo. `Cargo.lock` aparece excluido por `.gitignore`; por tanto, las versiones transitivas no quedan fijadas en el repositorio en este estado.
