; ==============================================================================
; KUNYIN Neo (Pure Rust Native) Windows NSIS Installer Script
; Zero WebView / Zero Electron / 100% Pure Rust Architecture
; ==============================================================================

Unicode true

!include "MUI2.nsh"
!include "FileFunc.nsh"

; General Definitions
!define PRODUCT_NAME "昆音 (Kunyin Neo)"
!define PRODUCT_VERSION "1.1.0"
!define PRODUCT_PUBLISHER "Kunyin Team"
!define PRODUCT_WEB_SITE "https://github.com/Combineqw/kunyin-neo"
!define PRODUCT_DIR_REGKEY "Software\Microsoft\Windows\CurrentVersion\App Paths\kunyin-desktop.exe"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\KunyinNeo"
!define PRODUCT_UNINST_ROOT_KEY "HKCU"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "..\..\..\dist\kunyin-rust-1.1.0-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\kunyin-desktop"
InstallDirRegKey ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_DIR_REGKEY}" ""
ShowInstDetails show
ShowUnInstDetails show
RequestExecutionLevel user

; MUI Interface Configuration
!define MUI_ABORTWARNING
!define MUI_ICON "${NSISDIR}\Contrib\Graphics\Icons\modern-install.ico"
!define MUI_UNICON "${NSISDIR}\Contrib\Graphics\Icons\modern-uninstall.ico"

; Installer Pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\kunyin-desktop.exe"
!define MUI_FINISHPAGE_RUN_TEXT "立即运行 昆音 (Kunyin Neo)"
!insertmacro MUI_PAGE_FINISH

; Uninstaller Pages
!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; Languages
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

; Installation Section
Section "MainSection" SEC01
  SetOutPath "$INSTDIR"
  SetOverwrite ifnewer

  ; Main pure Rust executable
  File "/oname=kunyin-desktop.exe" "..\target\release\aurora-app.exe"

  ; Create Shortcuts
  CreateDirectory "$SMPROGRAMS\昆音"
  CreateShortcut "$SMPROGRAMS\昆音\昆音 (Kunyin Neo).lnk" "$INSTDIR\kunyin-desktop.exe"
  CreateShortcut "$SMPROGRAMS\昆音\卸载 昆音.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\昆音.lnk" "$INSTDIR\kunyin-desktop.exe"

  ; Write Uninstaller
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  ; Register in Add/Remove Programs
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_DIR_REGKEY}" "" "$INSTDIR\kunyin-desktop.exe"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "UninstallString" "$INSTDIR\Uninstall.exe"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayIcon" "$INSTDIR\kunyin-desktop.exe"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"

  ; Register Audio File Associations (.mp3, .flac, .wav, .m4a, .ogg)
  WriteRegStr HKCU "Software\Classes\Kunyin.Audio\shell\open\command" "" '"$INSTDIR\kunyin-desktop.exe" "%1"'
  WriteRegStr HKCU "Software\Classes\.mp3\OpenWithProgids" "Kunyin.Audio" ""
  WriteRegStr HKCU "Software\Classes\.flac\OpenWithProgids" "Kunyin.Audio" ""
  WriteRegStr HKCU "Software\Classes\.wav\OpenWithProgids" "Kunyin.Audio" ""
  WriteRegStr HKCU "Software\Classes\.m4a\OpenWithProgids" "Kunyin.Audio" ""
  WriteRegStr HKCU "Software\Classes\.ogg\OpenWithProgids" "Kunyin.Audio" ""
SectionEnd

; Uninstallation Section
Section Uninstall
  ; Remove File Associations
  DeleteRegValue HKCU "Software\Classes\.mp3\OpenWithProgids" "Kunyin.Audio"
  DeleteRegValue HKCU "Software\Classes\.flac\OpenWithProgids" "Kunyin.Audio"
  DeleteRegValue HKCU "Software\Classes\.wav\OpenWithProgids" "Kunyin.Audio"
  DeleteRegValue HKCU "Software\Classes\.m4a\OpenWithProgids" "Kunyin.Audio"
  DeleteRegValue HKCU "Software\Classes\.ogg\OpenWithProgids" "Kunyin.Audio"
  DeleteRegKey HKCU "Software\Classes\Kunyin.Audio"

  ; Remove Shortcuts
  Delete "$DESKTOP\昆音.lnk"
  Delete "$SMPROGRAMS\昆音\昆音 (Kunyin Neo).lnk"
  Delete "$SMPROGRAMS\昆音\卸载 昆音.lnk"
  RMDir "$SMPROGRAMS\昆音"

  ; Remove Files and Directory
  Delete "$INSTDIR\kunyin-desktop.exe"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  ; Remove Registry Keys
  DeleteRegKey ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}"
  DeleteRegKey ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_DIR_REGKEY}"

  SetAutoClose true
SectionEnd
