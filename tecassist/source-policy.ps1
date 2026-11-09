$ErrorActionPreference='Stop'
$repo=Split-Path $PSScriptRoot -Parent
$connection=Get-Content (Join-Path $repo 'src\server\connection.rs') -Raw
$client=Get-Content (Join-Path $repo 'src\client.rs') -Raw
$resume=Get-Content (Join-Path $repo 'src\tec_assist_resume.rs') -Raw
$checks=[ordered]@{
    explicitInitialSelection=$connection.Contains('if files_selected && conn.authorized && conn.stream.is_secured()')
    activeScopeRechecked=([regex]::Matches($connection,'tec_assist_inherited_files && !crate::tec_assist_resume::files_alive').Count -eq 2)
    noSavedPasswordRequiredForTicket=$client.Contains('ConnType::FILE_TRANSFER && !crate::tec_assist_resume::file_credentials')
    fileProofDomainSeparated=$resume.Contains('TecAssist-FileTransfer-v25')
    deadlinePersisted=$resume.Contains('pub transfer_expires_at: u64')
    desktopScopeRetained=$connection.Contains('accept_proof(&self.lr, &self.hash, desktop)')
}
foreach($mode in @('console','standalone')){
    $passwordPage=Get-Content (Join-Path $repo "tecassist\overlays\$mode\flutter\lib\desktop\pages\desktop_home_page.dart") -Raw
    $checks["${mode}Password24Bold"]=$passwordPage.Contains('style: TextStyle(fontSize: 24, fontWeight: FontWeight.bold)')
}
$out=Join-Path $repo 'tecassist-out'
New-Item -ItemType Directory -Path $out -Force | Out-Null
$checks | ConvertTo-Json | Set-Content (Join-Path $out 'source-checks.json') -Encoding utf8
if($checks.Values -contains $false){throw 'Source policy checks failed'}
