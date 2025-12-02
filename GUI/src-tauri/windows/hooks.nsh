!macro NSIS_HOOK_POSTINSTALL
  ; Read existing user PATH
  ReadRegStr $0 HKCU "Environment" "PATH"

  ; Check if $INSTDIR is already in PATH
  StrStr $1 $0 $INSTDIR
  StrCmp $1 "" 0 +3
    ; Not found → append to PATH
    StrCmp $0 "" 0 +2
      StrCpy $0 "$INSTDIR"
      Goto +2
    StrCpy $0 "$0;$INSTDIR"
    WriteRegStr HKCU "Environment" "PATH" "$0"

    ; Refresh environment variables
    System::Call 'User32::SendMessageTimeoutA(i 0xffff,i 0x1a,i 0,i,"ptr",0,i 0,*i .r2)'

    ; Optional confirmation
    MessageBox MB_OK "Ninja directory added to PATH!"
!macroend