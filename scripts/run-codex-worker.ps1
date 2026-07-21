[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidatePattern('^[A-Za-z0-9_-]+$')][string]$Profile,
    [Parameter(Mandatory = $true)][string]$Worktree,
    [Parameter(Mandatory = $true)][string]$Packet,
    [ValidateSet('read-only', 'workspace-write')][string]$Sandbox = 'workspace-write',
    [string]$OutputLastMessage,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$worktreePath = (Resolve-Path -LiteralPath $Worktree).Path
$packetPath = (Resolve-Path -LiteralPath $Packet).Path

if (-not (Test-Path -LiteralPath (Join-Path $worktreePath '.git'))) {
    throw "Worktree is not a Git worktree: $worktreePath"
}
if ($packetPath.StartsWith($worktreePath + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Packet must be outside the target worktree.'
}

$codexHome = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $HOME '.codex' }
$profileFile = Join-Path $codexHome "$Profile.config.toml"
if (-not (Test-Path -LiteralPath $profileFile -PathType Leaf)) {
    throw "Required Codex profile overlay does not exist: $profileFile"
}
if (-not (Get-Command codex -ErrorAction SilentlyContinue)) {
    throw 'codex is not on PATH.'
}

$codexArgs = @('exec', '--profile', $Profile, '--sandbox', $Sandbox, '--cd', $worktreePath)
if ($OutputLastMessage) {
    $outputPath = [IO.Path]::GetFullPath($OutputLastMessage)
    $codexArgs += @('--output-last-message', $outputPath)
}
$codexArgs += '-'

if ($DryRun) {
    Write-Output "profile overlay: $profileFile"
    Write-Output "packet: $packetPath"
    Write-Output ('command: codex ' + ($codexArgs -join ' '))
    exit 0
}

Get-Content -LiteralPath $packetPath -Raw | & codex @codexArgs
exit $LASTEXITCODE
