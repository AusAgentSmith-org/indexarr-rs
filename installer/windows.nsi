; Indexarr Windows Installer
; Requires NSIS 3.x with Modern UI
; Build: makensis -DVERSION=x.y.z installer/windows.nsi

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

; ── Build-time variables ──────────────────────────────────────────────────────
!ifndef VERSION
  !define VERSION "dev"
!endif
!ifndef PG_VERSION
  !define PG_VERSION "17.4"
!endif

!define APP_NAME     "Indexarr"
!define APP_EXE      "indexarr.exe"
!define SVC_NAME     "Indexarr"
!define PG_SVC_NAME  "IndexarrPostgres"
!define PG_PORT      "5432"
!define PG_USER      "indexarr"
!define PG_DB        "indexarr"
!define PG_ZIP_URL   "https://get.enterprisedb.com/postgresql/postgresql-${PG_VERSION}-1-windows-x64-binaries.zip"

Name "${APP_NAME} ${VERSION}"
OutFile "indexarr-${VERSION}-windows-x86_64-setup.exe"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
InstallDirRegKey HKLM "Software\${APP_NAME}" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

; ── Pages ─────────────────────────────────────────────────────────────────────
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN      "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Open Indexarr in your browser when done"
!define MUI_FINISHPAGE_RUN_PARAMETERS "--workers http_server"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; ── Helpers ───────────────────────────────────────────────────────────────────

; Run a PowerShell command, abort on failure
!macro PSExec cmd
  nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -Command "${cmd}"'
  Pop $0
  ${If} $0 != 0
    MessageBox MB_OK|MB_ICONSTOP "Command failed (exit $0):$\r$\n${cmd}"
    Abort
  ${EndIf}
!macroend

; ── Install ───────────────────────────────────────────────────────────────────
Section "${APP_NAME}" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"

  ; ── 1. Install Indexarr binary ─────────────────────────────────────────────
  File "${APP_EXE}"

  ; ── 2. Download PostgreSQL binaries ───────────────────────────────────────
  ${If} ${RunningX64}
    DetailPrint "Downloading PostgreSQL ${PG_VERSION} binaries..."
    !insertmacro PSExec "[Net.ServicePointManager]::SecurityProtocol = 'Tls12,Tls13'; \
      Invoke-WebRequest -Uri '${PG_ZIP_URL}' \
        -OutFile '$TEMP\pgsql.zip' -UseBasicParsing"

    ; ── 3. Extract PostgreSQL ──────────────────────────────────────────────
    DetailPrint "Extracting PostgreSQL..."
    !insertmacro PSExec "Expand-Archive -Path '$TEMP\pgsql.zip' \
      -DestinationPath '$INSTDIR' -Force"
    ; The zip extracts to $INSTDIR\pgsql\

    ; ── 4. Init database cluster ───────────────────────────────────────────
    DetailPrint "Initialising database cluster..."
    CreateDirectory "$APPDATA\${APP_NAME}\pgdata"
    ; Generate a random superuser password (not exposed to users)
    !insertmacro PSExec "[System.Convert]::ToBase64String(\
      [System.Security.Cryptography.RandomNumberGenerator]::GetBytes(24)) \
      | Set-Content -NoNewline '$TEMP\pgpass.txt'"
    ; initdb
    !insertmacro PSExec "& '$INSTDIR\pgsql\bin\initdb.exe' \
      -D '$APPDATA\${APP_NAME}\pgdata' \
      -U postgres \
      --pwfile '$TEMP\pgpass.txt' \
      --encoding=UTF8 --locale=C"

    ; ── 5. Configure port ──────────────────────────────────────────────────
    !insertmacro PSExec "(Get-Content '$APPDATA\${APP_NAME}\pgdata\postgresql.conf') \
      -replace '#port = 5432', 'port = ${PG_PORT}' \
      | Set-Content '$APPDATA\${APP_NAME}\pgdata\postgresql.conf'"

    ; ── 6. Register and start PostgreSQL service ───────────────────────────
    DetailPrint "Registering PostgreSQL service..."
    !insertmacro PSExec "& '$INSTDIR\pgsql\bin\pg_ctl.exe' register \
      -N '${PG_SVC_NAME}' \
      -D '$APPDATA\${APP_NAME}\pgdata' \
      -S auto"
    nsExec::ExecToLog 'net start "${PG_SVC_NAME}"'
    Pop $0

    ; Wait for PostgreSQL to become ready (up to 30s)
    DetailPrint "Waiting for PostgreSQL to start..."
    StrCpy $1 0
    ${Do}
      Sleep 1000
      nsExec::ExecToStack '"$INSTDIR\pgsql\bin\pg_isready.exe" -p ${PG_PORT} -U postgres'
      Pop $0  ; exit code
      Pop $1  ; stdout
      IntOp $1 $1 + 1
      ${If} $1 >= 30
        MessageBox MB_OK|MB_ICONSTOP "PostgreSQL did not start within 30 seconds."
        Abort
      ${EndIf}
    ${LoopUntil} $0 == 0

    ; ── 7. Create indexarr role and database ──────────────────────────────
    DetailPrint "Creating Indexarr database..."
    ; Read superuser password
    !insertmacro PSExec "\
      $env:PGPASSFILE = '$TEMP\pgpass.txt'; \
      $pass = Get-Content '$TEMP\pgpass.txt'; \
      $env:PGPASSWORD = $pass; \
      & '$INSTDIR\pgsql\bin\psql.exe' -p ${PG_PORT} -U postgres \
        -c \"CREATE USER ${PG_USER} WITH PASSWORD '${PG_USER}';\" postgres; \
      & '$INSTDIR\pgsql\bin\psql.exe' -p ${PG_PORT} -U postgres \
        -c 'CREATE DATABASE ${PG_DB} OWNER ${PG_USER};' postgres"

    ; Clean up temp password file
    Delete "$TEMP\pgpass.txt"
    Delete "$TEMP\pgsql.zip"

    ; ── 8. Write .env ──────────────────────────────────────────────────────
    DetailPrint "Writing configuration..."
    FileOpen $0 "$INSTDIR\.env" w
    FileWrite $0 "INDEXARR_DB_URL=postgres://${PG_USER}:${PG_USER}@127.0.0.1:${PG_PORT}/${PG_DB}$\r$\n"
    FileWrite $0 "INDEXARR_DATA_DIR=$APPDATA\${APP_NAME}$\r$\n"
    FileClose $0

    ; ── 9. Register Indexarr as a Windows service ──────────────────────────
    DetailPrint "Registering Indexarr service..."
    nsExec::ExecToLog 'sc create "${SVC_NAME}" \
      binPath= "\"$INSTDIR\${APP_EXE}\" --all" \
      start= auto \
      DisplayName= "${APP_NAME}"'
    nsExec::ExecToLog 'sc description "${SVC_NAME}" "Decentralized torrent indexer"'
    nsExec::ExecToLog 'sc start "${SVC_NAME}"'

  ${Else}
    MessageBox MB_OK|MB_ICONSTOP "${APP_NAME} requires a 64-bit Windows system."
    Abort
  ${EndIf}

  ; ── 10. Shortcuts and registry ────────────────────────────────────────────
  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk" \
    "$INSTDIR\uninstall.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr  HKLM "Software\${APP_NAME}" "InstallDir" "$INSTDIR"
  WriteRegStr  HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "DisplayName"    "${APP_NAME}"
  WriteRegStr  HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegStr  HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "DisplayVersion"  "${VERSION}"
  WriteRegStr  HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "Publisher"       "${APP_NAME}"
  WriteRegStr  HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "DisplayIcon"     "$INSTDIR\${APP_EXE}"
SectionEnd

; ── Uninstall ─────────────────────────────────────────────────────────────────
Section "Uninstall"
  ; Stop and remove Indexarr service
  nsExec::ExecToLog 'sc stop "${SVC_NAME}"'
  nsExec::ExecToLog 'sc delete "${SVC_NAME}"'

  ; Stop and remove PostgreSQL service
  nsExec::ExecToLog 'net stop "${PG_SVC_NAME}"'
  nsExec::ExecToLog '"$INSTDIR\pgsql\bin\pg_ctl.exe" unregister -N "${PG_SVC_NAME}"'

  ; Remove files
  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\.env"
  Delete "$INSTDIR\uninstall.exe"
  RMDir /r "$INSTDIR\pgsql"
  RMDir "$INSTDIR"

  ; Shortcuts
  Delete "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk"
  RMDir  "$SMPROGRAMS\${APP_NAME}"

  ; Registry
  DeleteRegKey HKLM "Software\${APP_NAME}"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"

  ; Note: pgdata in %APPDATA% is intentionally left to preserve user data.
  MessageBox MB_OK "Indexarr has been removed.$\r$\nYour database is preserved at:$\r$\n  %APPDATA%\${APP_NAME}\pgdata"
SectionEnd
