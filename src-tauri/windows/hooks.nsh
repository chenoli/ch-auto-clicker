; CH. Clicker NSIS hooks (TASKS T12.2), included by the Tauri NSIS template.

!macro NSIS_HOOK_POSTUNINSTALL
  ; Starting with Windows must not point at an executable that no longer exists.
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "CH Clicker"
  ; Profiles and settings live outside the identifier folder Tauri removes, so they are
  ; deleted here, and only when the user ticks "Delete the application data".
  ${If} $DeleteAppDataCheckboxState = 1
    RMDir /r "$APPDATA\Chenoli\CH Clicker"
    RMDir /r "$LOCALAPPDATA\Chenoli\CH Clicker"
    RMDir "$APPDATA\Chenoli"
    RMDir "$LOCALAPPDATA\Chenoli"
  ${EndIf}
!macroend
