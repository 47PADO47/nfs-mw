<#
.SYNOPSIS
    Renders every HUD scenario offscreen and muted: the Custom HUD screenshot matrix.

.DESCRIPTION
    Runs `nfsmw view-world --drive --hud-scenario <name> --screenshot ...` once per scenario and pass:
    - 1280x720 (16:9), 2560x1080 (21:9) and 5120x1440 (32:9);
    - a high-contrast pass at 1280x720 (`--exec "set custom_hud.high_contrast on"`);
    - one pass per colour-vision palette at 1280x720 (skip them with -SkipPalettes).

    Every run is a screenshot run: the window is hidden, unfocused and off the desktop, and the game is muted, so the
    matrix never takes the focus or makes a sound. Each run is waited for as a process, never as a window.

    The pictures go to -OutDir, by default a new folder in the temporary directory; nothing is written into the
    repository. See docs/specs/custom-hud-scenarios.md.

.PARAMETER OutDir
    Where the PNGs go. Default: a new nfsmw-hud-shots-<time> folder in the temporary directory. A folder inside the
    repository is refused.

.PARAMETER Exe
    The nfsmw binary. Default: target/release/nfsmw.exe, else target/debug/nfsmw.exe.

.PARAMETER GameDir
    The install, passed as --game-dir. Default: the usual discovery (NFSMW_GAME_DIR, the config file, the registry).

.PARAMETER Scenario
    Only these scenarios. Default: all of them.

.PARAMETER SkipPalettes
    Leave out the colour-vision palette passes.

.EXAMPLE
    pwsh tools/hud_shots.ps1
    pwsh tools/hud_shots.ps1 -OutDir D:\shots -Scenario cooldown,racing
#>
[CmdletBinding()]
param(
    [string]$OutDir,
    [string]$Exe,
    [string]$GameDir,
    [string[]]$Scenario,
    [switch]$SkipPalettes
)

$ErrorActionPreference = 'Stop'

# The names of HudScenario::ALL (crates/nfsmw/src/devtools/hud_scenarios/mod.rs), in its order.
$AllScenarios = @(
    'free-roam', 'race-intro', 'racing', 'race-results',
    'pursuit-active', 'search', 'cooldown', 'escape',
    'critical-damage', 'notify-preempt', 'map-centered', 'map-expanded'
)
$Sizes = @('1280x720', '2560x1080', '5120x1440')
$Palettes = @('deuteranopia', 'tritanopia')
# The place the fixtures are built around (devtools/hud_scenarios/common.rs), with no traffic to move the background.
$World = @('view-world', '--drive', '--at', '2150,1399', '--no-traffic')
# Long enough after the scene is ready for the notices and the fades to settle (notify-preempt steps on frame 30).
$Delay = '1.5'

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

function Find-Exe {
    if ($Exe) {
        return (Resolve-Path $Exe).Path
    }
    foreach ($build in @('release', 'debug')) {
        $candidate = Join-Path $RepoRoot "target/$build/nfsmw.exe"
        if (Test-Path $candidate) {
            return $candidate
        }
    }
    throw 'no nfsmw binary: build one with `cargo build --release -p nfsmw`, or pass -Exe'
}

function Resolve-OutDir {
    if (-not $OutDir) {
        $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
        return Join-Path ([System.IO.Path]::GetTempPath()) "nfsmw-hud-shots-$stamp"
    }
    return [System.IO.Path]::GetFullPath($OutDir)
}

$binary = Find-Exe
$out = Resolve-OutDir
$separator = [System.IO.Path]::DirectorySeparatorChar
$repoPrefix = $RepoRoot.TrimEnd('\', '/') + $separator
if (($out.TrimEnd('\', '/') + $separator).StartsWith($repoPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "refusing to write screenshots inside the repository: $out"
}
New-Item -ItemType Directory -Force -Path $out | Out-Null

$names = $AllScenarios
if ($Scenario) {
    $unknown = $Scenario | Where-Object { $AllScenarios -notcontains $_ }
    if ($unknown) {
        throw "unknown scenario(s): $($unknown -join ', ') (expected: $($AllScenarios -join ', '))"
    }
    $names = $Scenario
}

# The passes: a file suffix, a size, and the console commands to run first.
$passes = @()
foreach ($size in $Sizes) {
    $passes += [pscustomobject]@{ Suffix = $size; Size = $size; Exec = @() }
}
$passes += [pscustomobject]@{ Suffix = 'high-contrast_1280x720'; Size = '1280x720'; Exec = @('set custom_hud.high_contrast on') }
if (-not $SkipPalettes) {
    foreach ($palette in $Palettes) {
        $passes += [pscustomobject]@{ Suffix = "${palette}_1280x720"; Size = '1280x720'; Exec = @("set custom_hud.palette $palette") }
    }
}

$failed = @()
$total = $names.Count * $passes.Count
$done = 0
foreach ($name in $names) {
    foreach ($pass in $passes) {
        $done++
        $file = Join-Path $out "${name}_$($pass.Suffix).png"
        $arguments = @()
        if ($GameDir) {
            $arguments += @('--game-dir', $GameDir)
        }
        $arguments += $World
        $arguments += @(
            '--ui-style', 'custom',
            '--hud-scenario', $name,
            '--screenshot', $file,
            '--screenshot-size', $pass.Size,
            '--screenshot-delay', $Delay,
            '--muted'
        )
        foreach ($command in $pass.Exec) {
            $arguments += @('--exec', $command)
        }
        Write-Host "[$done/$total] $name $($pass.Suffix)"
        # Run in this console and wait for the process: a screenshot run opens no visible window.
        & $binary @arguments
        if ($LASTEXITCODE -ne 0) {
            $failed += "${name} $($pass.Suffix) (exit $LASTEXITCODE)"
        }
    }
}

Write-Host "screenshots in $out"
if ($failed.Count -gt 0) {
    Write-Host "failed: $($failed -join '; ')"
    exit 1
}
