use std::fs;
use std::collections::HashMap;

fn main() {
    let content = fs::read_to_string("C:/Users/Sebastian/.local/share/opencode/tool-output/tool_152e14125001JTIjdLKvF6t7w7")
        .expect("Failed to read file");

    let pattern = regex::Regex::new(r#"\{\s*0x([0-9a-fA-F]+),\s*0x([0-9a-fA-F]+),\s*"([^"]+)"\s*\}"#).unwrap();

    let mut entries = Vec::new();
    let mut ext_counts: HashMap<String, usize> = HashMap::new();

    for cap in pattern.captures_iter(&content) {
        let sector = u32::from_str_radix(&cap[1], 16).unwrap();
        let size = u32::from_str_radix(&cap[2], 16).unwrap();
        let filename = cap[3].to_string();

        if sector == 0 && size == 0 && filename == "NULL" {
            continue;
        }

        entries.push(serde_json::json!({
            "filename": filename,
            "sector": sector,
            "size": size
        }));

        let ext = filename.split('.').last().unwrap_or("NOEXT").to_uppercase();
        *ext_counts.entry(ext).or_insert(0) += 1;
    }

    let json = serde_json::to_string_pretty(&entries).unwrap();
    fs::write("C:/Users/Sebastian/Desktop/Projects/Digimon-Asset-Extractor/data/file_table_us.json", json).unwrap();

    println!("Total entries: {}", entries.len());
    println!("\nFile types:");
    let mut sorted: Vec<_> = ext_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (ext, count) in sorted {
        println!("  .{}: {}", ext, count);
    }
}