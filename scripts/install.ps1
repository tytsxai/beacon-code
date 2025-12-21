#Requires -Version 5.1
<#
.SYNOPSIS
    Beacon Code Windows 安装脚本

.DESCRIPTION
    自动下载并安装 Beacon Code CLI

.PARAMETER Version
    指定版本号 (默认: latest)

.PARAMETER InstallDir
    安装目录 (默认: $env:USERPROFILE\.beacon-code)

.EXAMPLE
    .\install.ps1
    安装最新版本

.EXAMPLE
    .\install.ps1 -Version 0.6.0
    安装指定版本

.EXAMPLE
    .\install.ps1 -InstallDir C:\beacon-code
    自定义安装目录
#>

param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:USERPROFILE\.beacon-code"
)

$ErrorActionPreference = "Stop"

$GitHubRepo = "tytsxai/beacon-code"
$BinName = "code"
$Target = "x86_64-pc-windows-msvc"

function Write-Info { param($Message) Write-Host "[INFO] $Message" -ForegroundColor Green }
function Write-Warn { param($Message) Write-Host "[WARN] $Message" -ForegroundColor Yellow }
function Write-Err { param($Message) Write-Host "[ERROR] $Message" -ForegroundColor Red }

function Get-LatestVersion {
    $apiUrl = "https://api.github.com/repos/$GitHubRepo/releases/latest"
    try {
        $response = Invoke-RestMethod -Uri $apiUrl -UseBasicParsing
        return $response.tag_name -replace '^v', ''
    }
    catch {
        Write-Err "无法获取最新版本: $_"
        exit 1
    }
}

function Install-BeaconCode {
    Write-Info "检测到平台: Windows x64 ($Target)"

    # 确定版本
    if ($Version -eq "latest") {
        Write-Info "获取最新版本..."
        $Version = Get-LatestVersion
    }

    Write-Info "安装版本: v$Version"

    # 创建安装目录
    $binDir = Join-Path $InstallDir "bin"
    if (-not (Test-Path $binDir)) {
        New-Item -ItemType Directory -Path $binDir -Force | Out-Null
    }

    # 下载 URL
    $baseUrl = "https://github.com/$GitHubRepo/releases/download/v$Version"
    $archiveName = "$BinName-$Target.exe.zst"
    $downloadUrl = "$baseUrl/$archiveName"
    $binPath = Join-Path $binDir "$BinName.exe"

    # 临时目录
    $tmpDir = Join-Path $env:TEMP "beacon-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

    try {
        $archivePath = Join-Path $tmpDir $archiveName

        Write-Info "下载: $downloadUrl"
        Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing

        # 检查是否有 zstd
        $zstdPath = Get-Command zstd -ErrorAction SilentlyContinue

        if ($zstdPath) {
            Write-Info "使用 zstd 解压..."
            & zstd -d $archivePath -o $binPath --force
        }
        else {
            # 尝试下载 zip 格式
            Write-Warn "zstd 未安装，尝试下载 zip 格式..."
            $zipName = "$BinName-$Target.exe.zip"
            $zipUrl = "$baseUrl/$zipName"
            $zipPath = Join-Path $tmpDir $zipName

            Invoke-WebRequest -Uri $zipUrl -OutFile $zipPath -UseBasicParsing
            Expand-Archive -Path $zipPath -DestinationPath $tmpDir -Force

            $extractedExe = Join-Path $tmpDir "$BinName-$Target.exe"
            if (Test-Path $extractedExe) {
                Copy-Item $extractedExe $binPath -Force
            }
            else {
                Write-Err "解压失败，请安装 zstd: choco install zstd"
                exit 1
            }
        }

        Write-Info "安装完成: $binPath"

        # 添加到 PATH
        $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
        if ($userPath -notlike "*$binDir*") {
            Write-Info "添加到用户 PATH..."
            [Environment]::SetEnvironmentVariable(
                "PATH",
                "$binDir;$userPath",
                "User"
            )
            Write-Info "PATH 已更新，请重新打开终端"
        }

        Write-Host ""
        Write-Info "验证安装:"
        Write-Host "  $binPath --version"
        Write-Host ""
    }
    finally {
        # 清理临时目录
        if (Test-Path $tmpDir) {
            Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# 执行安装
Install-BeaconCode
