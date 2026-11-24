; NSIS per-user installer for KeyCat Native (Windows, zero WebView2).
; Build: makensis -INPUTCHARSET UTF8 -DVERSION=2.2.1 -DEXE_PATH=..\..\target\release\KeyCat.exe -DOUT_FILE=..\..\dist\KeyCat-Windows-Setup.exe installer.nsi

Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"

!ifndef VERSION
  !define VERSION "2.2.1"
!endif
!ifndef EXE_PATH
  !define EXE_PATH "..\..\target\release\KeyCat.exe"
!endif
!ifndef OUT_FILE
  !define OUT_FILE "..\..\dist\KeyCat-Windows-Setup.exe"
!endif

!define APP_NAME "KeyCat"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\KeyCat"

Name "${APP_NAME} ${VERSION}"
OutFile "${OUT_FILE}"
InstallDir "$PROGRAMFILES64\KeyCat"
InstallDirRegKey HKLM "${UNINSTALL_KEY}" "InstallLocation"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
ShowInstDetails nevershow
ShowUninstDetails nevershow

!define MUI_ICON "..\..\assets\icons\icon.ico"
!define MUI_UNICON "..\..\assets\icons\icon.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\KeyCat.exe"
!define MUI_FINISHPAGE_RUN_TEXT "立即运行 KeyCat"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "SimpChinese"

Section "Install"
  SetShellVarContext current
  SetOutPath "$INSTDIR"

  ; Stop any running copy before replacing the binary.
  nsExec::Exec 'taskkill /f /im KeyCat.exe'
  Pop $0

  ; Clean up legacy Electron 1.0 install if present, preserving settings.json.
  StrCpy $R8 "$LOCALAPPDATA\Programs\KeyCat"
  ${If} ${FileExists} "$R8\Uninstall KeyCat.exe"
    ExecWait '"$R8\Uninstall KeyCat.exe" /S _?=$R8'
    Delete "$R8\Uninstall KeyCat.exe"
    RMDir "$R8"
  ${EndIf}
  ${If} ${FileExists} "$APPDATA\KeyCat\Local State"
    ${IfNot} ${FileExists} "$APPDATA\com.ellobok.keycat\settings.json"
    ${AndIf} ${FileExists} "$APPDATA\KeyCat\settings.json"
      CreateDirectory "$APPDATA\com.ellobok.keycat"
      CopyFiles /SILENT "$APPDATA\KeyCat\settings.json" "$APPDATA\com.ellobok.keycat"
    ${EndIf}
    RMDir /r "$APPDATA\KeyCat"
  ${EndIf}

  ; Clean up legacy per-user install in %LOCALAPPDATA%\KeyCat (v2.0.0..v2.1.1) while keeping settings.
  StrCpy $R8 "$LOCALAPPDATA\KeyCat"
  ${If} $R8 != $INSTDIR
  ${AndIf} ${FileExists} "$R8\Uninstall.exe"
    ReadRegStr $R7 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "KeyCat"
    ExecWait '"$R8\Uninstall.exe" /S _?=$R8'
    Delete "$R8\Uninstall.exe"
    RMDir "$R8"
    DeleteRegKey HKCU "${UNINSTALL_KEY}"
    ${If} $R7 != ""
      WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "KeyCat" '"$INSTDIR\KeyCat.exe"'
    ${EndIf}
  ${EndIf}

  ; Remove any elevated login task that points to an older path outside $INSTDIR.
  ReadEnvStr $R6 USERNAME
  ${If} $R6 != ""
    nsExec::Exec 'schtasks /Delete /F /TN "KeyCat ($R6)"'
    Pop $0
  ${EndIf}

  SetShellVarContext all
  File "/oname=KeyCat.exe" "${EXE_PATH}"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  ; Start Menu shortcut
  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\KeyCat.exe" "" "$INSTDIR\KeyCat.exe" 0

  ; Update existing autostart Run entry if user already enabled it
  ReadRegStr $R9 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "KeyCat"
  ${If} $R9 != ""
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "KeyCat" '"$INSTDIR\KeyCat.exe"'
  ${EndIf}

  ; Apps & Features entry (per-machine)
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\KeyCat.exe,0"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "Publisher" "KeyCat"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoRepair" 1
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "EstimatedSize" 1024
SectionEnd

Section "Uninstall"
  nsExec::Exec 'taskkill /f /im KeyCat.exe'
  Pop $0

  ; Remove elevated scheduled task if present
  ReadEnvStr $R6 USERNAME
  ${If} $R6 != ""
    nsExec::Exec 'schtasks /Delete /F /TN "KeyCat ($R6)"'
    Pop $0
  ${EndIf}

  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "KeyCat"
  SetShellVarContext all
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"

  Delete "$INSTDIR\KeyCat.exe"
  Delete "$INSTDIR\keycat-task.xml"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegKey HKLM "${UNINSTALL_KEY}"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
SectionEnd
