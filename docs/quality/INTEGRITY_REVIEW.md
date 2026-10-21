# Revisión de integridad del proyecto

**Fecha:** 2026-10-21  
**Alcance:** lectura estática del árbol, manifiestos/configuración, ayuda de CLI y `cargo test --all-targets`. Esta revisión no modifica lógica de aplicación ni demuestra compatibilidad con un disco completo.

## Resultado de verificaciones

| Verificación | Resultado |
|---|---|
| `cargo test --all-targets` | Correcto: 59 pruebas pasan, 0 fallan. |
| CLI `dw-extract --help` | Correcto; se confirma la lista de subcomandos compilados. |
| Ayudas de `extract`, `convert-audio`, `convert-efe`, `convert-map`, `convert-scn` y `convert-str` | Consultadas correctamente en el binario compilado. |
| `convert-model` | No reconocido como subcomando. |
| Validación contra BIN/CUE de extremo a extremo | No ejecutada: no se usó un disco de juego como fixture de esta revisión. |
| Build de release | No ejecutado. |

Las pruebas pasan, pero el build muestra decenas de advertencias Rust de imports/variables/nombres no usados y una advertencia de incompatibilidad futura de `binrw 0.10`. El entorno también emitió un aviso al canonicalizar `C:\Users\Sebastian`. Estas advertencias no hicieron fallar las pruebas.

## Hallazgos que requieren corrección

### Alta — No hay verificación Redump efectiva; las referencias incrustadas son marcadores

En [`src/verification/mod.rs`](../src/verification/mod.rs), las entradas de `SYSTEM.CNF` y `CHDAT/ALLTIM.TIM` no contienen hashes de referencia fiables. El SHA256 de `ALLTIM.TIM` es el hash conocido de un archivo vacío (`e3b0…b855`), ambos MD5 usan el hash de archivo vacío (`d41d…8427e`) y el SHA256 de `SYSTEM.CNF` es una secuencia de apariencia artificial. **En el flujo de extracción observado, `Extractor::extract_file` no llama a `RedumpDatabase::verify` y asigna `NoReference` a los archivos**, así que esas referencias no autentican nada hoy; si se conectaran sin reemplazarlas, producirían estados engañosos.

**Acción recomendada:** retirar esas entradas o sustituirlas por datos contrastados con una fuente Redump verificable; después conectar la verificación con pruebas. Hasta entonces, describir hashes solo como integridad interna del manifiesto.

### Alta — `extract-files` no aplica la opción `--no-verify`

La CLI declara `--no-verify`, pero `run_extract_files` no recibe el valor y construye el `Extractor` con `verify_formats = false` de forma fija. La validación de formato queda desactivada para extracción selectiva con o sin esa opción.

**Acción recomendada:** pasar el valor a `run_extract_files` y probar ambos casos, o retirar la opción de ese comando hasta implementarla.

### Media — `convert-tim` contradice su ayuda en modo de archivo/directorio

En [`src/cli/mod.rs`](../src/cli/mod.rs), el argumento `--recursive` no se usa: el recorrido de directorios es siempre recursivo. Para un archivo individual, la función entrega el argumento `--output` directamente a `to_png` aunque la ayuda lo define como directorio de salida. Además, los errores de conversión se imprimen y se absorben, por lo que el proceso puede terminar con código cero aunque la conversión falle.

**Acción recomendada:** definir salida para un archivo individual, respetar `--recursive` y devolver error/código no cero si falla una conversión.

### Media — El parser CUE no admite de forma fiable nombres BIN entrecomillados con espacios

`parse_cue` separa la línea `FILE` usando espacios en blanco y toma el segundo fragmento como ruta. Una línea normal como `FILE "Digimon World.bin" BINARY` se recorta a `Digimon`. Las extracciones CLI requieren además `--bin` explícito, pero la resolución de respaldo basada en la ruta del CUE y la selección de pares en GUI pueden fallar con esos nombres.

**Acción recomendada:** analizar la sintaxis CUE respetando comillas y agregar una prueba con espacios en la ruta.

### Media — La extracción de texturas MAP y el OBJ son prototipos

En [`src/formats/map/map_yaml.rs`](../src/formats/map/map_yaml.rs), `export_map_textures` declara que necesita offsets del TIM original y no extrae texturas. El OBJ genera un plano aproximado a partir de tiles/colisión y algunos puntos de entidades; no representa la geometría completa del mapa. `convert-map` no ofrece una conversión TFS independiente a pesar de que existe un lector TFS.

**Acción recomendada:** etiquetar claramente las salidas actuales como aproximadas y no anunciar extracción de texturas hasta implementar y probar su localización real.

### Media — STR entrega bitstreams MDEC, no fotogramas reproducibles

La salida de `--extract-video` son archivos `frame_*.mdec` con datos comprimidos. No hay decodificador de vídeo ni mux a MP4. La descripción del comando puede inducir a pensar que produce vídeo visible.

**Acción recomendada:** mantener el nombre/ayuda como extracción de bitstream hasta añadir una ruta de decodificación validada.

### Media — Las pistas STR requieren validación de extracción extremo a extremo

El extractor ahora tiene una ruta STR que vuelca sectores crudos completos de 2352 bytes sin filtrar submode; los formatos normales siguen usando 2048 bytes de datos de usuario. El tamaño de la tabla (2336 bytes por sector) determina cuántos sectores leer y el manifiesto registra los bytes realmente escritos. La extracción y conversión del BIN real aún deben compararse con el volcado manual para cerrar la validación extremo a extremo.

**Acción recomendada:** completar la comprobación de aceptación del ciclo 3.1 del [plan](../planning/INTEGRITY_REMEDIATION_PLAN.md): comparar el WAV extraído con un golden manual completo e independiente.

### Media — El soporte SCN está implementado, pero no validado contra el juego

`src/formats/script/` ya está declarado desde `src/formats/mod.rs` y `convert-scn` existe en la CLI: lee la tabla de scripts, desensambla, decodifica texto con la tabla del charset y exporta YAML o listado. Las pruebas son unitarias y usan scripts sintéticos construidos con `test_support::ScriptBuilder`; no se ha contrastado la salida con un `DG.SCN` real ni con el comportamiento del intérprete del juego, y el YAML reproduce el formato de la referencia C++ sin ser un formato oficial del juego.

**Acción recomendada:** ejecutar `convert-scn` sobre los scripts extraídos del disco y comparar el desensamblado con el intérprete de `dw-engine` antes de anunciar el formato como compatible.

### Baja/Media — Las dependencias no están fijadas por `Cargo.lock`

`.gitignore` excluye `Cargo.lock`, así que las versiones resueltas pueden variar entre builds. Rust recomienda conservar el lockfile para aplicaciones/binarios cuando se busca reproducibilidad.

**Acción recomendada:** decidir explícitamente si este repositorio versiona `Cargo.lock`; si sí, quitar esa regla y revisar el archivo generado.

## Estado limpio frente a estado funcional

La tabla de archivos y las pruebas unitarias pasan sus comprobaciones actuales. Eso no valida todos los sectores, rutas, hashes de referencia ni la fidelidad de las conversiones. La matriz de capacidades y sus límites está en [Formatos](../reference/FORMATS.md); los comandos documentados están en [CLI](../reference/CLI.md).

El ciclo 3.1 añadió extracción de sectores STR crudos y su comprobación con `OP1.STR`; H7 sigue en validación hasta comparar el WAV con un golden independiente. Los hallazgos restantes se gestionan por ciclo en el [Plan de remediación de integridad](../planning/INTEGRITY_REMEDIATION_PLAN.md).
