param(
    [Parameter(Mandatory = $true)][string]$SdkDirectory,
    [Parameter(Mandatory = $true)][string]$ReleaseDirectory
)

$ErrorActionPreference = 'Stop'
$sdk = (Resolve-Path -LiteralPath $SdkDirectory).Path
$release = (Resolve-Path -LiteralPath $ReleaseDirectory).Path
$bin = Join-Path $sdk 'bin\x64'
if (-not (Test-Path -LiteralPath (Join-Path $release 'nfsmw.exe') -PathType Leaf)) {
    throw 'The release directory must already contain a native-dlss-enabled nfsmw.exe.'
}
$dlls = @(
    'sl.interposer.dll', 'sl.common.dll', 'sl.dlss.dll', 'sl.pcl.dll', 'sl.reflex.dll',
    'sl.dlss_g.dll', 'NvLowLatencyVk.dll', 'nvngx_dlss.dll', 'nvngx_dlssg.dll'
)
$files = @()
foreach ($name in $dlls) {
    $source = Join-Path $bin $name
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Missing SDK runtime: $source" }
    if ($name -like 'sl.*.dll') {
        $version = (Get-Item -LiteralPath $source).VersionInfo
        if ($version.FileMajorPart -ne 2 -or $version.FileMinorPart -ne 14 -or $version.FileBuildPart -ne 16) {
            throw "Streamline runtime version must match the 2.14.16 SDK used by this build: $source"
        }
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $source
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=NVIDIA Corporation') {
        throw "Runtime is not signed by NVIDIA with a valid signature: $source"
    }
    $files += @{ Source = $source; Name = $name }
}
$files += @(
    @{ Source = (Join-Path $sdk 'license.txt'); Name = 'streamline.license.txt' },
    @{ Source = (Join-Path $bin 'nvngx_dlss.license.txt'); Name = 'nvngx_dlss.license.txt' },
    @{ Source = (Join-Path $bin 'reflex.license.txt'); Name = 'reflex.license.txt' }
)
foreach ($file in $files) {
    if (-not (Test-Path -LiteralPath $file.Source -PathType Leaf)) { throw "Missing license: $($file.Source)" }
    if (Test-Path -LiteralPath (Join-Path $release $file.Name)) { throw "Destination already exists: $($file.Name)" }
}
foreach ($file in $files) {
    Copy-Item -LiteralPath $file.Source -Destination (Join-Path $release $file.Name)
}
Write-Output "Copied signed Streamline runtime and NVIDIA notices to $release."
