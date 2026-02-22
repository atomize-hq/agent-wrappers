$ErrorActionPreference = "Stop"

Write-Host "## Agent API Codex stream_exec parity smoke (windows)"
rustc --version
cargo --version

$root = Resolve-Path (Join-Path $PSScriptRoot "..\\..\\..\\..\\..\\..")
Set-Location $root

$tmp = Join-Path $env:TEMP ("agent-api-codex-smoke-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

try {
  $fakeBin = Join-Path $tmp "fakebin"
  New-Item -ItemType Directory -Force -Path $fakeBin | Out-Null

  $fakePs1 = Join-Path $fakeBin "fake_codex.ps1"
  @'
$ErrorActionPreference = "Stop"

$lastMessage = $null
$schema = $null
for ($i = 0; $i -lt $args.Count; $i++) {
  if ($args[$i] -eq "--output-last-message" -and ($i + 1) -lt $args.Count) {
    $lastMessage = $args[$i + 1]
    $i++
    continue
  }
  if ($args[$i] -eq "--output-schema" -and ($i + 1) -lt $args.Count) {
    $schema = $args[$i + 1]
    $i++
    continue
  }
}

try { $null = [Console]::In.ReadToEnd() } catch {}

Write-Output '{"type":"thread.started","thread_id":"thread-1"}'
Write-Output '{"type":"turn.started","thread_id":"thread-1","turn_id":"turn-1"}'
Write-Output '{"type":"item.started","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-1","item_type":"agent_message","content":{"text":"hello from fake codex"}}'
Write-Output '{"type":"turn.completed","thread_id":"thread-1","turn_id":"turn-1"}'

if ($lastMessage) {
  New-Item -ItemType Directory -Force -Path (Split-Path $lastMessage) | Out-Null
  Set-Content -Path $lastMessage -Value "hello from fake codex" -NoNewline
}

if ($schema) {
  New-Item -ItemType Directory -Force -Path (Split-Path $schema) | Out-Null
  Set-Content -Path $schema -Value "{}"
}

if ($env:CODEX_WRAPPER_SMOKE_DUMP_ENV) {
  New-Item -ItemType Directory -Force -Path (Split-Path $env:CODEX_WRAPPER_SMOKE_DUMP_ENV) | Out-Null
  Get-ChildItem Env: | Sort-Object Name | ForEach-Object { "$($_.Name)=$($_.Value)" } | Set-Content -Path $env:CODEX_WRAPPER_SMOKE_DUMP_ENV
}
'@ | Set-Content -Path $fakePs1 -Encoding UTF8

  $fakeCmd = Join-Path $fakeBin "codex.cmd"
  @"
@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0fake_codex.ps1" %*
"@ | Set-Content -Path $fakeCmd -Encoding ASCII

  $env:CODEX_HOME = (Join-Path $tmp "codex-home")
  New-Item -ItemType Directory -Force -Path $env:CODEX_HOME | Out-Null

  # Cover both spawn strategies:
  # - wrapper picks up `CODEX_BINARY`
  # - direct spawn uses `codex` from `PATH`
  $env:CODEX_BINARY = $fakeCmd
  $env:PATH = "$fakeBin;$env:PATH"

  Write-Host "Running required tests (fixture/fake-binary only)"
  cargo test -p agent_api --all-features
  cargo test -p agent_api --features codex

  if ($env:RUN_WORKSPACE_ALL -eq "1") {
    Write-Host "RUN_WORKSPACE_ALL=1: running broader workspace tests (may be slow)"
    cargo test --workspace --all-targets --all-features
  }

  Write-Host "OK"
} finally {
  Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $tmp
}

