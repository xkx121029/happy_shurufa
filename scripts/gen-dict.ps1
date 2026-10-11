# Dev-time tool: generate embedded dictionaries from mozillazg/pinyin-data
# Usage: powershell -File scripts/gen-dict.ps1 -Phrase <phrase_pinyin.txt> -Chars <pinyin_data.txt>
# Output: crates/happy-core/res/pinyin.dict (key = toneless pinyin, values joined by '|', sorted by key)
#         crates/happy-core/res/syllables.txt (all valid toneless syllables, sorted & unique)
param(
    [Parameter(Mandatory = $true)][string]$Phrase,
    [Parameter(Mandatory = $true)][string]$Chars
)
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$resDir = Join-Path $root "crates\happy-core\res"
New-Item -ItemType Directory -Force $resDir | Out-Null

# Tone letters -> plain letters (build from code points, keep script pure ASCII)
$plainSets = @(
    @{ base = 'a'; codes = @(0x0101, 0x00E1, 0x01CE, 0x00E0) },
    @{ base = 'e'; codes = @(0x0113, 0x00E9, 0x011B, 0x00E8) },
    @{ base = 'i'; codes = @(0x012B, 0x00ED, 0x01D0, 0x00EC) },
    @{ base = 'o'; codes = @(0x014D, 0x00F3, 0x01D2, 0x00F2) },
    @{ base = 'u'; codes = @(0x016B, 0x00FA, 0x01D4, 0x00F9) },
    @{ base = 'v'; codes = @(0x01D6, 0x01D8, 0x01DA, 0x01DC) },
    @{ base = 'v'; codes = @(0x00FC) },
    @{ base = 'n'; codes = @(0x0144, 0x0148, 0x01F9) },
    @{ base = 'm'; codes = @(0x1E3F) },
    @{ base = 'e'; codes = @(0x00EA) }
)
$map = @{}
foreach ($set in $plainSets) {
    foreach ($c in $set.codes) { $map[[string][char]$c] = $set.base }
}
function Convert-ToPlain([string]$s) {
    $sb = [System.Text.StringBuilder]::new()
    foreach ($ch in $s.ToCharArray()) {
        $key = [string]$ch
        if ($map.ContainsKey($key)) { [void]$sb.Append($map[$key]) } else { [void]$sb.Append($ch) }
    }
    return $sb.ToString().ToLowerInvariant()
}

# Phrases: "word: pin1 pin2", alternative readings separated by " / "
$phraseMap = [System.Collections.Generic.Dictionary[string, System.Collections.Generic.List[string]]]::new()
$syllables = [System.Collections.Generic.HashSet[string]]::new()
foreach ($line in [IO.File]::ReadLines($Phrase)) {
    if ($line.StartsWith('#') -or -not $line.Contains(':')) { continue }
    $idx = $line.IndexOf(':')
    $word = $line.Substring(0, $idx).Trim()
    if ($word.Length -lt 2) { continue }
    $readings = $line.Substring($idx + 1).Split('/')
    foreach ($r in $readings) {
        $tokens = $r.Trim() -split '\s+' | Where-Object { $_ }
        if ($tokens.Count -eq 0) { continue }
        $plain = ($tokens | ForEach-Object { Convert-ToPlain $_ }) -join ''
        if (-not $plain) { continue }
        if (-not $phraseMap.ContainsKey($plain)) { $phraseMap[$plain] = [System.Collections.Generic.List[string]]::new() }
        if (-not $phraseMap[$plain].Contains($word)) { [void]$phraseMap[$plain].Add($word) }
        foreach ($t in $tokens) { [void]$syllables.Add((Convert-ToPlain $t)) }
    }
}

# Single chars: "U+4E00: yi # char"
$charMap = [System.Collections.Generic.Dictionary[string, System.Collections.Generic.List[string]]]::new()
foreach ($line in [IO.File]::ReadLines($Chars)) {
    if ($line.StartsWith('#') -or -not $line.StartsWith('U+')) { continue }
    try {
        $m = [regex]::Match($line, '^U\+([0-9A-Fa-f]{4,6}):([^#]+)')
        if (-not $m.Success) { continue }
        $cp = [Convert]::ToInt32($m.Groups[1].Value, 16)
        if ($cp -lt 0x3400) { continue }  # skip BMP symbols/latin blocks
        $ch = [char]::ConvertFromUtf32($cp)
        foreach ($raw in ($m.Groups[2].Value -split ',')) {
            $plain = Convert-ToPlain $raw.Trim()
            if (-not $plain) { continue }
            if (-not $charMap.ContainsKey($plain)) { $charMap[$plain] = [System.Collections.Generic.List[string]]::new() }
            if (-not $charMap[$plain].Contains($ch)) { [void]$charMap[$plain].Add($ch) }
            [void]$syllables.Add($plain)
        }
    } catch { continue }
}

# Merge output: phrases first (sorted by length), then chars
$out = [System.Collections.Generic.List[string]]::new()
$keys = [System.Collections.Generic.HashSet[string]]::new()
foreach ($k in $phraseMap.Keys) { [void]$keys.Add($k) }
foreach ($k in $charMap.Keys) { [void]$keys.Add($k) }
$sortedKeys = $keys | Sort-Object { $_ }
foreach ($k in $sortedKeys) {
    $parts = [System.Collections.Generic.List[string]]::new()
    if ($phraseMap.ContainsKey($k)) {
        $phrases = [string[]]@($phraseMap[$k] | ForEach-Object { $_ })
        $parts.AddRange([string[]]($phrases | Sort-Object { $_.Length }))
    }
    if ($charMap.ContainsKey($k)) { $parts.AddRange([string[]]@($charMap[$k] | ForEach-Object { $_ })) }
    $joined = $parts -join '|'
    if ($joined) { $out.Add("$k`t$joined") }
}
[IO.File]::WriteAllLines((Join-Path $resDir "pinyin.dict"), $out)
[IO.File]::WriteAllLines((Join-Path $resDir "syllables.txt"), ($syllables | Sort-Object { $_ }))
Write-Host "dict entries: $($out.Count)  syllables: $($syllables.Count)"
