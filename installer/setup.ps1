#Requires -Version 5.1
# Indexarr PostgreSQL setup script — bundled inside the NSIS installer
# Called by the installer with the paths it chose; never run directly by users.
param(
    [string]$InstallDir,
    [string]$DataDir,
    [string]$PgVersion = "17.4",
    [string]$PgPort    = "5432",
    [string]$PgUser    = "indexarr",
    [string]$PgDb      = "indexarr"
)

$ErrorActionPreference = "Stop"
$pgBin  = "$InstallDir\pgsql\bin"
$pgData = "$DataDir\pgdata"

function Log($msg) { Write-Host "[indexarr-setup] $msg" }

# ── 1. Download PostgreSQL binaries ──────────────────────────────────────────
$zipUrl = "https://get.enterprisedb.com/postgresql/postgresql-$PgVersion-1-windows-x64-binaries.zip"
$zipTmp = "$env:TEMP\indexarr-pgsql.zip"
Log "Downloading PostgreSQL $PgVersion..."
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]'Tls12,Tls13'
Invoke-WebRequest -Uri $zipUrl -OutFile $zipTmp -UseBasicParsing
Log "Extracting..."
Expand-Archive -Path $zipTmp -DestinationPath $InstallDir -Force
Remove-Item $zipTmp -ErrorAction SilentlyContinue

# ── 2. Initialise cluster ─────────────────────────────────────────────────────
Log "Initialising database cluster at $pgData..."
New-Item -ItemType Directory -Force -Path $pgData | Out-Null
$pgPass = [System.Convert]::ToBase64String(
    [System.Security.Cryptography.RandomNumberGenerator]::GetBytes(24))
$passTmp = "$env:TEMP\indexarr-pgpass.txt"
$pgPass | Set-Content -NoNewline $passTmp

& "$pgBin\initdb.exe" -D $pgData -U postgres --pwfile $passTmp --encoding=UTF8 --locale=C
if ($LASTEXITCODE -ne 0) { throw "initdb failed with exit code $LASTEXITCODE" }

# ── 3. Configure port ─────────────────────────────────────────────────────────
(Get-Content "$pgData\postgresql.conf") -replace '#port = 5432', "port = $PgPort" |
    Set-Content "$pgData\postgresql.conf"

# ── 4. Register + start PostgreSQL service ────────────────────────────────────
Log "Registering PostgreSQL service..."
& "$pgBin\pg_ctl.exe" register -N IndexarrPostgres -D $pgData -S auto
if ($LASTEXITCODE -ne 0) { throw "pg_ctl register failed" }
Start-Service IndexarrPostgres

# Wait up to 30s for readiness
Log "Waiting for PostgreSQL to be ready..."
$ready = $false
for ($i = 0; $i -lt 30; $i++) {
    Start-Sleep -Seconds 1
    & "$pgBin\pg_isready.exe" -p $PgPort -U postgres 2>$null
    if ($LASTEXITCODE -eq 0) { $ready = $true; break }
}
if (-not $ready) { throw "PostgreSQL did not become ready within 30 seconds" }

# ── 5. Create role + database ─────────────────────────────────────────────────
Log "Creating database..."
$env:PGPASSWORD = $pgPass
& "$pgBin\psql.exe" -p $PgPort -U postgres postgres -c "CREATE USER $PgUser WITH PASSWORD '$PgUser';"
if ($LASTEXITCODE -ne 0) { throw "Failed to create role $PgUser" }
& "$pgBin\psql.exe" -p $PgPort -U postgres postgres -c "CREATE DATABASE $PgDb OWNER $PgUser;"
if ($LASTEXITCODE -ne 0) { throw "Failed to create database $PgDb" }
Remove-Item $env:PGPASSWORD -ErrorAction SilentlyContinue
Remove-Item $passTmp -ErrorAction SilentlyContinue

# ── 6. Write .env ─────────────────────────────────────────────────────────────
Log "Writing configuration..."
@"
INDEXARR_DB_URL=postgres://${PgUser}:${PgUser}@127.0.0.1:${PgPort}/${PgDb}
INDEXARR_DATA_DIR=${DataDir}
"@ | Set-Content "$InstallDir\.env" -Encoding UTF8

Log "Setup complete."
