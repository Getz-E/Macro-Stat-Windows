; SPDX-License-Identifier: GPL-3.0-or-later
; Copyright (C) 2026 MacroStat contributors
;
; MacroStat — kullanıcı bazlı (yönetici istemeyen) Inno Setup kurulumu.
; Derleme:  iscc /DMyAppVersion=1.0.0 windows\installer\MacroStat.iss
; Çıktı:    windows\dist\MacroStat-Setup-<sürüm>.exe
;
; İlkeler:
;  * PrivilegesRequired=lowest — yükseltme istenmez; kurulum %LOCALAPPDATA% altına gider.
;  * Kaldırma yalnızca kendi dosyalarını siler; kullanıcı verisi (profil,
;    günlük, ayar) yalnızca açık onayla silinir.
;  * İmzalama opsiyoneldir; imzasız kurulumda SmartScreen uyarısı belgelenir.

#define MyAppName "MacroStat"
#define MyAppPublisher "MacroStat contributors"
#define MyAppExeName "MacroStat.exe"
#define MyAppUrl "https://github.com/Getz-E/Macro-Stat"

#ifndef MyAppVersion
  #define MyAppVersion "1.0.0"
#endif

[Setup]
AppId={{6F1B7E42-3C9A-4D2E-9B5A-1D8F0C7A21E4}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppUrl}
AppSupportURL={#MyAppUrl}
AppUpdatesURL={#MyAppUrl}
VersionInfoVersion={#MyAppVersion}
DefaultDirName={localappdata}\Programs\MacroStat
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=..\dist
OutputBaseFilename=MacroStat-Setup-{#MyAppVersion}
SetupIconFile=..\gui\assets\macrostat.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64compatible
ArchitecturesAllowed=x64compatible
MinVersion=10.0.19045
CloseApplications=yes
RestartApplications=no
DirExistsWarning=no
AllowNoIcons=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "turkish"; MessagesFile: "compiler:Languages\Turkish.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\dist\MacroStat\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\THIRD_PARTY.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
{ Kaldırmada kullanıcı verisini yalnızca açık onayla sil. }
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  DataDir: String;
  SettingsDir: String;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    DataDir := ExpandConstant('{localappdata}\MacroStat');
    SettingsDir := ExpandConstant('{appdata}\MacroStat');
    if (DirExists(DataDir) or DirExists(SettingsDir)) and
       (MsgBox('Also delete your MacroStat profiles, settings and logs?' + #13#10 +
               'This permanently removes:' + #13#10 + DataDir + #13#10 +
               SettingsDir + #13#10 + #13#10 +
               'Choose No to keep them for a later reinstall.',
               mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES) then
    begin
      DelTree(DataDir, True, True, True);
      DelTree(SettingsDir, True, True, True);
    end;
  end;
end;
