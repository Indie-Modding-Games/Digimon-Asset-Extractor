# Digimon World Asset Extractor

`dw-extract` extrae archivos de la imagen BIN/CUE de **Digimon World (USA), SLUS-01032**. Incluye una CLI, una GUI y módulos Rust para inspeccionar o convertir algunos formatos del juego.

> La extracción de archivos ordinarios conserva los datos de usuario de sectores MODE2 Form 1. Las pistas STR se extraen como sectores crudos MODE2/2352 para conservar sus cabeceras y poder procesarlas con `convert-str`. Los conversores tienen distintos niveles de cobertura; consulta [Formatos](docs/reference/FORMATS.md) y [Revisión de integridad](docs/quality/INTEGRITY_REVIEW.md).

## Compilar

Requisitos: Rust y Cargo instalados.

```powershell
cargo build --release --bin dw-extract
cargo build --release --bin dw-extract-gui
```

Los ejecutables se generan en `target/release/`. Para ejecutar desde el código:

```powershell
cargo run --bin dw-extract -- --help
cargo run --bin dw-extract-gui
```

El paquete define varios binarios; incluye `--bin dw-extract` al usar `cargo run` para la CLI.

## Uso rápido

### Extraer todos los archivos

```powershell
dw-extract extract --bin game.bin --cue game.cue --output assets --manifest manifest.json
```

La CLI comprueba la versión mediante `SYSTEM.CNF`, extrae los archivos enumerados en la tabla estadounidense integrada y genera hashes cuando se solicita un manifiesto. La extracción completa puede devolver código de salida distinto de cero si encuentra errores.

### Extraer archivos concretos

```powershell
dw-extract extract-files --bin "Digimon World.bin" --cue "Digimon World.cue" --output assets AGUM.MMD CHDAT/ALLTIM.TIM
```

La tabla actual contiene 818 entradas: 178 archivos MMD y 52 TIM. Consulta la [referencia de CLI](docs/reference/CLI.md) y sus notas sobre las opciones de validación.

### Consultar la tabla integrada

```powershell
dw-extract info
dw-extract list --mmd
dw-extract list --tim
```

### Abrir la interfaz gráfica

```powershell
dw-extract-gui
```

La GUI permite seleccionar o arrastrar un par BIN/CUE, elegir extracción básica o completa, configurar la estructura de salida y ver el progreso. La GUI está enfocada en extraer archivos; las conversiones descritas en la CLI no aparecen como paneles de conversión.

## Documentación del proyecto

- [Índice de documentación](docs/README.md): mapa de guías, referencias, arquitectura y planes.
- [Guía de comandos](docs/reference/CLI.md): argumentos, ejemplos y limitaciones actuales.
- [Arquitectura](docs/architecture/ARCHITECTURE.md): flujo BIN/CUE, módulos y responsabilidades.
- [Formatos](docs/reference/FORMATS.md): diferencia entre extracción, módulos disponibles y conversión expuesta.
- [Revisión de integridad](docs/quality/INTEGRITY_REVIEW.md): verificaciones ejecutadas y hallazgos que requieren atención.
- [Plan de soporte de formatos](docs/planning/FORMAT_SUPPORT_PLAN.md): trabajo futuro, sujeto a su protocolo de ciclos y aprobación.
- [Características](FEATURES.md): resumen compacto del estado observado.

## Estado del proyecto

El flujo de extracción BIN/CUE y varios lectores/conversores están implementados, pero la cobertura es desigual. En particular, los módulos MAP/OBJ y STR/MDEC tienen limitaciones que impiden interpretarlos como exportación final del mapa o como vídeo reproducible. La comparación contra hashes Redump no está conectada al flujo de extracción y las referencias actuales son marcadores. Lee la [revisión de integridad](docs/quality/INTEGRITY_REVIEW.md) para el detalle.

## Licencia y datos del juego

El paquete declara licencia MIT. Este repositorio no reemplaza la necesidad de tener una copia legal de la imagen del juego. La tabla de archivos se basa en el proyecto [dw_decomp](https://github.com/jype0/dw_decomp).
