!include "FileFunc.nsh"
!include "WordFunc.nsh"
!insertmacro WordFind

!macro NSIS_HOOK_POSTINSTALL
  ; Read existing user PATH
  ReadRegStr $0 HKCU "Environment" "PATH"

  ; Check if $INSTDIR is already in PATH
  ; WordFind params: output_var, string, word, case_sensitive, delimiter
  ${WordFind} $1 $0 $INSTDIR 0 ";"

  ; $1 = -1 → not found
  ${If} $1 == -1
    ; Append $INSTDIR to PATH
    StrCmp $0 "" 0 +2
      StrCpy $0 "$INSTDIR"
      Goto +3
    StrCpy $0 "$0;$INSTDIR"
    WriteRegStr HKCU "Environment" "PATH" "$0"

    ; Refresh environment variables
    System::Call 'User32::SendMessageTimeoutA(i 0xffff,i 0x1a,i 0,i,"ptr",0,i 0,*i .r2)'

    ; Optional confirmation
    MessageBox MB_OK "Ninja directory added to PATH!"
  ${EndIf}
!macroend
