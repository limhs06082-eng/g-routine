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
Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip

$installer = Get-ChildItem (Join-Path $root "src-tauri/target/release/bundle/nsis") -Filter "*.exe" | Select-Object -First 1
if ($installer) { Copy-Item $installer.FullName $out }
Get-ChildItem $out -File | ForEach-Object { "{0}  {1:N1} MB" -f $_.Name, ($_.Length / 1MB) }
