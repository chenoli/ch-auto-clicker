!macro NSIS_HOOK_POSTINSTALL
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
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "CH Clicker"
  DeleteRegKey HKCU "Software\Chenoli\CH Clicker"
  DeleteRegKey /ifempty HKCU "Software\Chenoli"
  ${If} $DeleteAppDataCheckboxState = 1
    RMDir /r "$APPDATA\Chenoli\CH Clicker"
    RMDir /r "$LOCALAPPDATA\Chenoli\CH Clicker"
    RMDir "$APPDATA\Chenoli"
    RMDir "$LOCALAPPDATA\Chenoli"
  ${EndIf}
!macroend
