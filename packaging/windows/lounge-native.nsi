!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef SRC
  !define SRC "..\..\dist"
!endif
!ifndef OUTFILE
  !define OUTFILE "lounge-native-setup.exe"
!endif

!define APP_NAME "Lounge Native"
!define APP_EXE "lounge-native.exe"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\LoungeNative"

Name "${APP_NAME}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\Lounge Native"
InstallDirRegKey HKCU "Software\Lounge Native" "InstallDir"
RequestExecutionLevel user
Unicode true
SetCompressor /SOLID lzma
Icon "${SRC}\lounge-native.ico"
UninstallIcon "${SRC}\lounge-native.ico"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "FileDescription" "${APP_NAME} installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "MIT"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File "${SRC}\lounge-native.exe"
  File "${SRC}\lounge-native-app.exe"
  File "${SRC}\README.md"
  File "${SRC}\LICENSE"
  File "${SRC}\lounge-native.ico"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateDirectory "$SMPROGRAMS"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\lounge-native.ico" 0

  WriteRegStr HKCU "Software\Lounge Native" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\lounge-native.ico"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "Lounge Native contributors"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\lounge-native.exe"
  Delete "$INSTDIR\lounge-native-app.exe"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\lounge-native.ico"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${APP_NAME}.lnk"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
  DeleteRegKey HKCU "Software\Lounge Native"
SectionEnd

Function .onInstSuccess
  IfSilent done
  ReadEnvStr $0 "GSTREAMER_1_0_ROOT_MSVC_X86_64"
  IfFileExists "$0\bin\gstreamer-1.0-0.dll" done
  IfFileExists "$0\bin\gstreamer-1.0.dll" done
  ReadEnvStr $0 "GSTREAMER_ROOT_X86_64"
  IfFileExists "$0\bin\gstreamer-1.0-0.dll" done
  IfFileExists "$0\bin\gstreamer-1.0.dll" done
  IfFileExists "C:\gstreamer\1.0\msvc_x86_64\bin\gstreamer-1.0-0.dll" done
  IfFileExists "$PROGRAMFILES64\gstreamer\1.0\msvc_x86_64\bin\gstreamer-1.0-0.dll" done
  MessageBox MB_YESNO|MB_ICONQUESTION "Lounge Native needs the GStreamer 1.26 runtime for audio and video. Open the GStreamer download page now?" IDNO done
  ExecShell "open" "https://gstreamer.freedesktop.org/download/#windows"
done:
FunctionEnd
