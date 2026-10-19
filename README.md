# Digimon World Asset Extractor (dw-extract)

A standalone tool to extract assets from **Digimon World (USA) SLUS-01032** BIN/CUE disc images.

Based on the file table from the [dw_decomp](https://github.com/jype0/dw_decomp) project.

## Features

- Extracts all game assets from BIN/CUE images (MODE2/2352 sectors)
- Validates SLUS-01032 version via SYSTEM.CNF
- Extracts key files: `CHDAT/ALLTIM.TIM` (texture archive), all 54 `.MMD` model files
- Generates SHA256/MD5 manifest with format validation
- Verifies extracted assets against manifest
- Safe extraction: skips unchanged files, refuses to overwrite modified files without `--force`
- Format validation for TIM (texture) and MMD (model) files

## Installation

### From Source (Rust)

```bash
git clone https://github.com/your-repo/dw-extract
cd dw-extract
cargo build --release
# Binary at ./target/release/dw-extract.exe
```

### Pre-built Binary

Download the latest release from the releases page.

## Usage

### Extract All Assets

```bash
dw-extract extract --bin "Digimon World (USA).bin" --cue "Digimon World (USA).cue" --output assets
```

### Extract Specific Files

```bash
dw-extract extract-files --bin "Digimon World (USA).bin" --cue "Digimon World (USA).cue" --output assets AGUM.MMD AIRD.MMD CHDAT/ALLTIM.TIM
```

### List Files in File Table

```bash
# All files
dw-extract list

# Only MMD files
dw-extract list --mmd

# Only TIM files
dw-extract list --tim
```

### Verify Extracted Assets

```bash
dw-extract verify --manifest manifest.json --assets assets
```

### Show File Table Info

```bash
dw-extract info
```

### Generate Manifest During Extraction

```bash
dw-extract extract --bin "..." --cue "..." --output assets --manifest manifest.json
```

## Command Reference

### `extract`
Extract all files from the disc image.

| Option | Description |
|--------|-------------|
| `--bin, -b` | Path to .bin file (required) |
| `--cue, -c` | Path to .cue file (required) |
| `--output, -o` | Output directory (default: `assets`) |
| `--force` | Overwrite existing files even if modified |
| `--no-verify` | Skip TIM/MMD format validation |
| `--manifest` | Generate manifest JSON at specified path |

### `extract-files`
Extract specific files by name.

| Option | Description |
|--------|-------------|
| `--bin, -b` | Path to .bin file (required) |
| `--cue, -c` | Path to .cue file (required) |
| `--output, -o` | Output directory (default: `assets`) |
| `files` | File names to extract (required, space-separated) |
| `--force` | Overwrite existing files even if modified |
| `--no-verify` | Skip format validation |
| `--manifest` | Generate manifest JSON |

### `list`
List files in the internal file table.

| Option | Description |
|--------|-------------|
| `--mmd` | Show only MMD model files |
| `--tim` | Show only TIM texture files |

### `verify`
Verify extracted assets against a manifest.

| Option | Description |
|--------|-------------|
| `--manifest, -m` | Path to manifest JSON (required) |
| `--assets, -a` | Assets directory (default: `assets`) |

### `info`
Show file table information and supported version.

## Output Structure

```
assets/
├── SYSTEM.CNF
├── CARD.ALL
├── CHDAT/
│   └── ALLTIM.TIM
├── AGUM.MMD
├── AIRD.MMD
├── ... (54 MMD files total)
```

## Supported Version

- **SLUS-01032** (Digimon World USA) - Full support
- Other versions (SLES, SLPS) - Rejected with clear error message

## File Formats

### TIM (Texture)
- Validates header ID (0x10)
- Checks BPP (4, 8, 16, 24)
- Reports dimensions and CLUT presence

### MMD (Model)
- Validates TMD header ID ('TMD=')
- Counts objects and detects embedded texture

## Verification

The tool generates a manifest with:
- SHA256 and MD5 hashes for each file
- File size
- Format validation status
- Verification status (Verified/Mismatch/NoReference)

Currently uses embedded Redump references for SYSTEM.CNF and ALLTIM.TIM. Other files show "NoReference" status.

## Notes

- **EDC Mismatches**: The tool reports EDC checksum mismatches as warnings but continues extraction. The user data is still correctly extracted as verified by the `verify` command.
- **Safe Extraction**: By default, the tool will not overwrite existing files that differ from the source. Use `--force` to override.
- **Sector Format**: Handles MODE2/2352 XA sectors with Form 1 data (2048 bytes user data per sector).

## License

MIT