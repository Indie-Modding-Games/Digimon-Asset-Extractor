# Estado de características

**Revisado:** 2026-10-21. Este resumen refleja el árbol y la CLI actuales; presencia de código no implica cobertura validada con todos los archivos del juego.

## Disponible en la interfaz

- Extracción completa o por nombre desde la tabla BIN/CUE de SLUS-01032.
- Interfaz gráfica de extracción con selección/arrastre BIN/CUE, progreso, log y configuración de salida.
- CLI para extraer, listar, consultar conteos, comparar con manifiesto y convertir algunos formatos.
- Organización plana, YAML configurable y mapeo especial CHDAT.
- Hashes SHA256/MD5 en manifiestos.

## Comandos de conversión

| Comando | Salida ofrecida | Límite actual |
|---|---|---|
| `convert-tim` | PNG | La salida para archivo individual, `--recursive` y reporte de fallo requieren corrección. |
| `convert-audio` | WAV, raw, SF2 o MIDI | Cobertura real no probada sobre un corpus completo. MIDI escribe un archivo por pista de un music font y uno por secuencia suelta. |
| `convert-efe` | YAML o JSON; bloques TMD/TIM opcionales | No ejecuta ni renderiza scripts de efectos. |
| `convert-map` | YAML u OBJ aproximado | OBJ no reconstruye el mapa completo; texturas no se extraen aún. |
| `convert-scn` | YAML o listado de desensamblado | No interpreta los scripts; la validación se hizo con scripts sintéticos, no con el disco completo. |
| `convert-str` | bitstreams MDEC y/o audio WAV XA | El lector de sectores y el decodificador XA replican la referencia y se comprobaron con la pista OP1.STR del BIN real. No decodifica fotogramas a imagen ni produce MP4. |

Para los argumentos, consulta la [referencia CLI](docs/reference/CLI.md). Para saber qué módulos están expuestos y qué está planeado, consulta [Formatos](docs/reference/FORMATS.md).

## Pruebas actuales

`cargo test --all-targets`: 103 pruebas pasan en el entorno revisado (98 de biblioteca y 5 de integración end-to-end de la CLI). La suite cubre principalmente unidades y fixtures sintéticos; no valida el disco completo ni todos los formatos. El build presenta advertencias de compilación. Ver [Revisión de integridad](docs/quality/INTEGRITY_REVIEW.md).

## Pendiente de atención

Los puntos siguientes están planificados por ciclo en el [plan de remediación de integridad](docs/planning/INTEGRITY_REMEDIATION_PLAN.md), con su comprobación de aceptación:

- Sustituir o retirar hashes Redump que son valores de marcador.
- Conectar correctamente `--no-verify` en `extract-files`.
- Corregir contrato de salida/recursión/errores de `convert-tim`.
- Completar extracción de texturas MAP y definir claramente el alcance de OBJ.
- Añadir decodificación MDEC si se requiere vídeo visible.
- Contrastar `convert-scn` con los scripts reales del disco.
- Validar `convert-audio` (WAV/SF2/MIDI) contra bancos de sonido reales del juego.
- Decidir si se versiona `Cargo.lock` para fijar las dependencias.
- Respetar las comillas en el parser de CUE (rutas de BIN con espacios).
- Extraer las pistas STR desde el BIN con el extractor normal.
