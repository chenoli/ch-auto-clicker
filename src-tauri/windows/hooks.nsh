; CH. Clicker NSIS hooks (TASKS T12.2), included by the Tauri NSIS template.

!macro NSIS_HOOK_POSTINSTALL
  ; Interactive installs only: updates run passively (/P) or silently and must keep the language
  ; the user chose in the app.
  ${If} $PassiveMode <> 1
  ${AndIfNot} ${Silent}
    ${If} $LANGUAGE = 1046
      WriteRegStr HKCU "Software\Chenoli\CH Clicker" "PendingLocale" "pt-BR"
    ${ElseIf} $LANGUAGE = 1034
      WriteRegStr HKCU "Software\Chenoli\CH Clicker" "PendingLocale" "es-419"
    ${Else}
      WriteRegStr HKCU "Software\Chenoli\CH Clicker" "PendingLocale" "en-US"
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Starting with Windows must not point at an executable that no longer exists.
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "CH Clicker"
  DeleteRegKey HKCU "Software\Chenoli\CH Clicker"
  DeleteRegKey /ifempty HKCU "Software\Chenoli"
  ; Profiles and settings live outside the identifier folder Tauri removes, so they are
  ; deleted here, and only when the user ticks "Delete the application data".
  ${If} $DeleteAppDataCheckboxState = 1
    RMDir /r "$APPDATA\Chenoli\CH Clicker"
    RMDir /r "$LOCALAPPDATA\Chenoli\CH Clicker"
    RMDir "$APPDATA\Chenoli"
    RMDir "$LOCALAPPDATA\Chenoli"
  ${EndIf}
!macroend
