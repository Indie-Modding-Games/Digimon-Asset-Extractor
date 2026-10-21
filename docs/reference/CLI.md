# Referencia de la CLI

La CLI se compila como el binario `dw-extract`. Para consultar la sintaxis exacta instalada, usa `dw-extract --help` o `dw-extract <comando> --help`.

## Comandos

### `extract`

Extrae todos los archivos de la tabla integrada.

```text
dw-extract extract --bin <archivo.bin> --cue <archivo.cue> [--output assets]
    [--force] [--no-verify] [--manifest <archivo.json>]
    [--structured] [--structure-config <archivo.yaml>] [--flat] [--preserve-chdat]
```

- `--output`: directorio base; por omisión `assets`.
- `--force`: permite reemplazar archivos existentes distintos del origen.
- `--no-verify`: desactiva la validación de formato TIM/MMD durante extracción completa.
- `--manifest`: escribe hashes y resultados en JSON.
- `--structured`: usa el mapeo integrado de `config/extraction_structure.yaml`.
- `--structure-config`: proporciona otro YAML de estructura.
- `--flat`: prevalece sobre estructura y guarda en la raíz de salida.
- `--preserve-chdat`: coloca `ALLTIM.TIM` en la raíz y distribuye MMD según el mapeo CHDAT conocido.

### `extract-files`

```text
dw-extract extract-files --bin <archivo.bin> --cue <archivo.cue> [opciones de extract] <archivo>...
```

Acepta nombres o rutas de la tabla, por ejemplo `AGUM.MMD CHDAT/ALLTIM.TIM`. Las entradas `.STR` se vuelcan como sectores crudos íntegros de 2352 bytes para conservar sync, cabecera y subheader que necesita `convert-str`; el manifiesto refleja el tamaño del volcado, que puede diferir del tamaño de tabla (contado a 2336 bytes por sector). Los demás archivos siguen usando datos de usuario de 2048 bytes por sector. La validación de formatos sigue desactivada para este comando independientemente de `--no-verify`.

### `list` e `info`

```text
dw-extract list [--mmd | --tim]
dw-extract info
```

`list` muestra nombre, sector inicial y tamaño de la tabla incorporada. `info` muestra los conteos para SLUS-01032 (818 entradas, 178 MMD y 52 TIM en la tabla actual).

### `verify`

```text
dw-extract verify --manifest <archivo.json> [--assets assets]
```

Vuelve a calcular SHA256 y MD5 de cada ruta del manifiesto y los compara con los valores guardados allí. Esto detecta cambios posteriores a la creación del manifiesto; no autentica el disco contra una base externa.

### `convert-tim`

```text
dw-extract convert-tim --input <archivo.tim-o-directorio> --output <ruta> [--recursive]
```

El decodificador de biblioteca puede convertir TIM de 4, 8, 16 y 24 bpp a PNG. **La integración CLI necesita corrección:** `--recursive` no cambia el recorrido del directorio y, con un archivo individual, `--output` se pasa como ruta del PNG aunque la ayuda lo describe como directorio. La CLI captura fallos de conversión y puede terminar con código de éxito. Hasta corregirlo, usa el decodificador desde la biblioteca o verifica manualmente el archivo de salida.

### `convert-audio`

```text
dw-extract convert-audio --input <VHB-o-VAB-o-SEQ> --output <directorio>
    [--format wav|raw|sf2|midi] [--sample-rate 37800] [--extract-vags] [--recursive]
```

El comando intenta leer VHB/VAB con el lector nuevo y tiene un lector VHB legado de respaldo. `raw` extrae flujos; `wav` decodifica a WAV; `sf2` crea un SoundFont a partir del VAB y sus muestras VAG; `midi` escribe los archivos MIDI de las secuencias. `--recursive` habilita el recorrido de subdirectorios cuando la entrada es un directorio.

- Con `--format midi`, un archivo de music font (VHB con banco de secuencias) produce un MIDI por pista: `track_00.mid`, `track_01.mid`, …; una secuencia suelta (`.seq`) produce `<nombre>.mid`. Si la entrada es un directorio con varios archivos, cada uno recibe su propio subdirectorio en la salida.
- Con `--format sf2`, la salida es `output.sf2` en el directorio indicado.
- `--sample-rate` solo afecta al WAV; el SoundFont usa 44100 Hz, como el chip de sonido.

La conversión no cuenta con pruebas de regresión sobre un corpus completo del juego.

### `convert-efe`

```text
dw-extract convert-efe --input <archivo.efe> --output <directorio>
    [--format yaml|json] [--extract-assets]
```

Escribe `efe.yaml` o `efe.json`; con `--extract-assets` también escribe los bloques TMD/TIM encontrados. No ejecuta el VM de efectos ni renderiza los efectos.

### `convert-map`

```text
dw-extract convert-map --input <archivo.map> --output <directorio>
    [--format yaml|obj|both] [--extract-textures]
    [--images-8 <n>] [--images-4 <n>] [--npcs true|false]
```

El lector de MAP permite inferir algunas opciones si no se especifican todas. **La implementación OBJ es una aproximación del plano de colisión y entidades**, no una reconstrucción completa de geometría del juego. La extracción de texturas está incompleta y actualmente solo informa que requiere offsets de TIM del archivo original. Aunque existe un lector TFS en la biblioteca, este comando no ofrece una ruta de conversión TFS independiente.

### `convert-scn`

```text
dw-extract convert-scn --input <archivo.scn> --output <directorio>
    [--format yaml|disasm]
```

Lee un script SCN: si el archivo es un contenedor con tabla de scripts (p. ej. `DG.SCN`) escribe un archivo por script (`script_0000.yaml`, `script_0001.yaml`, …); si es un script individual escribe `script.yaml`. `yaml` produce la estructura del script (etiquetas, instrucciones, condiciones, texto y datos); `disasm` produce el listado de instrucciones. La decodificación de texto usa la tabla del charset del juego, con las sustituciones de la fuente.

### `convert-str`

```text
dw-extract convert-str --input <archivo.str> --output <directorio>
    [--extract-video] [--extract-audio]
```

Se debe pasar al menos una opción. `--extract-video` guarda fotogramas como flujos comprimidos `.mdec`; **no** crea imágenes decodificadas ni MP4. `--extract-audio` decodifica pistas XA y escribe WAV.

El archivo de entrada debe ser el flujo de sectores crudos del disco (2352 bytes por sector: sync, cabecera, subheader, datos de usuario y corrección de errores). Una vista de 2048 bytes por sector pierde el audio y los tipos de sector, así que no se puede leer.

## Comandos que no existen

La CLI actual no incluye `convert-model`. El exportador de modelos no está implementado como comando.
