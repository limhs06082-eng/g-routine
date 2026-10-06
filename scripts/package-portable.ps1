$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$version = (Get-Content (Join-Path $root "package.json") -Raw | ConvertFrom-Json).version
$exe = Join-Path $root "src-tauri/target/release/g-routine.exe"
if (-not (Test-Path $exe)) { throw "먼저 'npm run tauri build'를 실행하세요." }

$out = Join-Path $root "release"
$stage = Join-Path $out "G-routine-portable"
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force (Join-Path $stage "data") | Out-Null
Copy-Item $exe (Join-Path $stage "G-routine.exe")
Set-Content -Path (Join-Path $stage "data/README.txt") -Encoding utf8 -Value "이 폴더에 G-routine 데이터가 저장됩니다. 프로그램과 함께 옮기세요."

$zip = Join-Path $out "G-routine_${version}_portable.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
# 항목 이름에 항상 '/'를 쓴다. PS 5.1의 Compress-Archive/CreateFromDirectory는 '\'를 써서 일부 압축 해제 프로그램이 data 폴더를 만들지 못한다.
$archive = [System.IO.Compression.ZipFile]::Open($zip, [System.IO.Compression.ZipArchiveMode]::Create)
try {
    $prefixLen = (Resolve-Path $stage).Path.TrimEnd('\').Length + 1
    Get-ChildItem $stage -Recurse -File | ForEach-Object {
        $entryName = $_.FullName.Substring($prefixLen).Replace('\', '/')
        [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, $_.FullName, $entryName, [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
    }
} finally { $archive.Dispose() }

# 이번 버전의 설치 파일만 복사한다 (bundle/nsis에는 예전 버전 파일도 남아 있을 수 있다)
$installer = Join-Path $root "src-tauri/target/release/bundle/nsis/G-routine_${version}_x64-setup.exe"
if (Test-Path $installer) { Copy-Item $installer $out }
Get-ChildItem $out -File | ForEach-Object { "{0}  {1:N1} MB" -f $_.Name, ($_.Length / 1MB) }
