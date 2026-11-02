; NSIS installer for BaseCreator. Build with:
;   makensis -DVERSION=0.0.4 -DOUTFILE=basecreator-setup.exe packaging/windows/installer.nsi
; (run from the repository root; the release build must exist in target\release)
!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef OUTFILE
  !define OUTFILE "basecreator-setup.exe"
!endif

!define APP "BaseCreator"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}"

Name "${APP} ${VERSION}"
OutFile "${OUTFILE}"
Unicode True
InstallDir "$PROGRAMFILES64\${APP}"
InstallDirRegKey HKLM "Software\${APP}" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

!define MUI_ICON "..\..\assets\basecreator.ico"
!define MUI_UNICON "..\..\assets\basecreator.ico"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\basecreator.exe"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File "..\..\target\release\basecreator.exe"
  File "..\..\LICENSE"
  File "..\..\LICENSE-APACHE"
  File "..\..\README.md"
  CreateDirectory "$SMPROGRAMS\${APP}"
  CreateShortcut "$SMPROGRAMS\${APP}\${APP}.lnk" "$INSTDIR\basecreator.exe"
  CreateShortcut "$SMPROGRAMS\${APP}\Uninstall ${APP}.lnk" "$INSTDIR\uninstall.exe"
  CreateShortcut "$DESKTOP\${APP}.lnk" "$INSTDIR\basecreator.exe"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKLM "Software\${APP}" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayName" "${APP}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\basecreator.exe"
  WriteRegStr HKLM "${UNINST_KEY}" "Publisher" "Michal Sarna"
  WriteRegStr HKLM "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\basecreator.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\LICENSE-APACHE"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${APP}\${APP}.lnk"
  Delete "$SMPROGRAMS\${APP}\Uninstall ${APP}.lnk"
  RMDir "$SMPROGRAMS\${APP}"
  Delete "$DESKTOP\${APP}.lnk"
  DeleteRegKey HKLM "${UNINST_KEY}"
  DeleteRegKey HKLM "Software\${APP}"
SectionEnd
