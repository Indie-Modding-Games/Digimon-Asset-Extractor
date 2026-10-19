$content = Get-Content -Raw "C:/Users/Sebastian/.local/share/opencode/tool-output/tool_152e5c8e6001QXh08c59QigWw" 2>&1

# Try alternative path
if (-not $content) {
    $content = Get-Content -Raw "C:/Users/Sebastian/.local/share/opencode/tool-output/*file_table*" 2>&1
}

$pattern = '\{ 0x([0-9a-fA-F]+), 0x([0-9a-fA-F]+), "([^"]+)" \}'
$matches = [System.Text.RegularExpressions.Regex]::Matches($content, $pattern)

$entries = @()
foreach ($match in $matches) {
    $sector = [Convert]::ToInt32($match.Groups[1].Value, 16)
    $size = [Convert]::ToInt32($match.Groups[2].Value, 16)
    $filename = $match.Groups[3].Value
    
    if ($sector -eq 0 -and $size -eq 0 -and $filename -eq "NULL") { continue }
    
    $entries += @{ filename = $filename; sector = $sector; size = $size }
}

$json = $entries | ConvertTo-Json -Depth 3 -Compress
$json | Out-File -Encoding utf8 "C:/Users/Sebastian/Desktop/Projects/Digimon-Asset-Extractor/data/file_table_us.json"

Write-Host "Total entries: $($entries.Count)"

$exts = $entries | Group-Object { ($_.filename -split '\.')[-1].ToUpper() } | Sort-Object Count -Descending
Write-Host "File types:"
$exts | ForEach-Object { Write-Host "  .$($_.Name): $($_.Count)" }