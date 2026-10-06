# G-routine 릴리스: 버전 반영 → 서명 빌드 → 포터블 zip → latest.json → 커밋·태그·push → GitHub 릴리스
# 사용법: powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Version 0.1.1 -Notes "바뀐 점"
# 서명 키(~/.tauri/g-routine.key)가 있어야 한다. 이 키를 잃어버리면 이미 설치된 앱에 업데이트를 보낼 수 없다.
param(
  [Parameter(Mandatory = $true)][string]$Version,
  [string]$Notes = ""
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Fail($msg) { throw "릴리스를 멈췄어요: $msg" }
function Run($label, [scriptblock]$cmd) {
  Write-Output "== $label"
  & $cmd
  if ($LASTEXITCODE -ne 0) { Fail "$label 단계가 실패했어요 (exit $LASTEXITCODE)" }
}

if ($Version -notmatch '^\d+\.\d+\.\d+$') { Fail "버전은 0.1.1 형식으로 입력하세요." }
$keyPath = Join-Path $env:USERPROFILE ".tauri\g-routine.key"
if (-not (Test-Path $keyPath)) { Fail "서명 키가 없어요: $keyPath" }
if ((git rev-parse --abbrev-ref HEAD) -ne "main") { Fail "main 브랜치에서 실행하세요." }
if (git status --porcelain --untracked-files=no) { Fail "커밋하지 않은 변경이 있어요. 먼저 커밋하세요." }
if (git tag --list "v$Version") { Fail "v$Version 태그가 이미 있어요." }
$repo = (gh repo view --json nameWithOwner -q .nameWithOwner)
if (-not $repo) { Fail "GitHub 저장소를 찾지 못했어요 (gh auth / origin 확인)." }

# 1. 버전 반영 (package.json + package-lock.json, tauri.conf.json, Cargo.toml)
Run "버전 $Version 반영" { npm version $Version --no-git-tag-version --allow-same-version | Out-Null }
function Set-FirstMatch($file, $pattern, $replacement) {
  $text = [IO.File]::ReadAllText($file)
  $re = [regex]::new($pattern, [Text.RegularExpressions.RegexOptions]::Multiline)
  if (-not $re.IsMatch($text)) { Fail "$file 에서 버전을 찾지 못했어요." }
  [IO.File]::WriteAllText($file, $re.Replace($text, $replacement, 1))
}
Set-FirstMatch "src-tauri/tauri.conf.json" '"version": "\d+\.\d+\.\d+"' "`"version`": `"$Version`""
Set-FirstMatch "src-tauri/Cargo.toml" '^version = "\d+\.\d+\.\d+"' "version = `"$Version`""

# 2. 서명 빌드 (설치 파일 + .sig)
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content $keyPath -Raw)
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
Run "서명 빌드" { npm run tauri build }
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY, Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD

$setupName = "G-routine_${Version}_x64-setup.exe"
$setup = Join-Path $root "src-tauri/target/release/bundle/nsis/$setupName"
if (-not (Test-Path "$setup.sig")) { Fail "서명 파일($setupName.sig)이 만들어지지 않았어요." }

# 3. 포터블 zip
Run "포터블 zip" { powershell -ExecutionPolicy Bypass -File (Join-Path $root "scripts/package-portable.ps1") }

# 4. latest.json (앱이 확인하는 업데이트 정보)
$latest = [ordered]@{
  version   = $Version
  notes     = $(if ($Notes) { $Notes } else { "G-routine v$Version" })
  pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
  platforms = [ordered]@{
    "windows-x86_64" = [ordered]@{
      signature = (Get-Content "$setup.sig" -Raw).Trim()
      url       = "https://github.com/$repo/releases/download/v$Version/$setupName"
    }
  }
}
$latestPath = Join-Path $root "release/latest.json"
[IO.File]::WriteAllText($latestPath, ($latest | ConvertTo-Json -Depth 5))

# 5. 커밋 · 태그 · push
Run "버전 커밋" { git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock }
if (git status --porcelain --untracked-files=no) {
  Run "버전 커밋" { git commit -q -m "chore: release v$Version" }
}
Run "태그" { git tag "v$Version" }
Run "push" { git push origin main "v$Version" }

# 6. GitHub 릴리스 (latest로 표시 → 앱이 latest.json을 받아 감)
$zip = Join-Path $root "release/G-routine_${Version}_portable.zip"
Run "GitHub 릴리스" {
  gh release create "v$Version" $setup "$setup.sig" $zip $latestPath `
    --repo $repo --title "G-routine v$Version" --notes $latest.notes --latest
}
Write-Output "완료: https://github.com/$repo/releases/tag/v$Version"
