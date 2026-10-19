#!/usr/bin/env python3
import re
import json

# Read the C file content
with open('C:/Users/Sebastian/.local/share/opencode/tool-output/tool_152e5c8e6001QXh08c59QigWw', 'r', encoding='utf-8', errors='ignore') as f:
    content = f.read()

# Extract all file entries
pattern = r'\{\s*0x([0-9a-fA-F]+),\s*0x([0-9a-fA-F]+),\s*"([^"]+)"\s*\}'
matches = re.findall(pattern, content)

entries = []
for sector_hex, size_hex, filename in matches:
    sector = int(sector_hex, 16)
    size = int(size_hex, 16)
    if sector == 0 and size == 0 and filename == "NULL":
        continue
    entries.append({
        "filename": filename,
        "sector": sector,
        "size": size
    })

# Write JSON
with open('C:/Users/Sebastian/Desktop/Projects/Digimon-Asset-Extractor/data/file_table_us.json', 'w') as f:
    json.dump(entries, f, indent=2)

print(f"Total entries: {len(entries)}")

# Group by extension
from collections import Counter
exts = Counter()
for e in entries:
    ext = e['filename'].split('.')[-1].upper() if '.' in e['filename'] else 'NOEXT'
    exts[ext] += 1

print("\nFile types:")
for ext, count in exts.most_common():
    print(f"  .{ext}: {count}")