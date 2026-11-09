param([ValidateSet('All','Checks')][string]$Phase='All')
$ErrorActionPreference='Stop'
$repo=Split-Path $PSScriptRoot -Parent
$out=Join-Path $repo 'tecassist-out'
New-Item -ItemType Directory -Path $out -Force | Out-Null
$env:RUSTUP_TOOLCHAIN='1.75.0'
$env:CI='true'
function Run([string]$program,[string[]]$arguments,[string]$log){
    & $program @arguments *> (Join-Path $out $log)
    if($LASTEXITCODE -ne 0){Get-Content (Join-Path $out $log) -Tail 60;throw "$program failed: $log"}
}
Push-Location $repo
try{
    & "$PSScriptRoot\source-policy.ps1"
    foreach($name in @('tec_assist_consent','tec_assist_resume_policy')){
        $exe=Join-Path $out "$name-tests.exe"
        Run 'rustc' @('--test',"src/$name.rs",'-o',$exe) "$name-compile.log"
        Run $exe @() "$name-tests.log"
    }
    if($Phase -eq 'Checks'){return}
    Run 'cargo' @('build','--locked','--release','--lib','--features','flutter') 'native-build.log'
    $native=Join-Path $repo 'target\release\librustdesk.dll'
    if($env:CARGO_TARGET_DIR){$native=Join-Path $env:CARGO_TARGET_DIR 'release\librustdesk.dll'}
    Copy-Item -LiteralPath $native -Destination (Join-Path $out 'librustdesk.dll') -Force
    $compiler=Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
    $helper=Join-Path $PSScriptRoot 'helpers'
    foreach($test in @($false,$true)){
        $exe=if($test){'helper-test.exe'}else{'TecAssist-PreparaRipresa.exe'}
        $manifest=if($test){'diagnostic-test.manifest'}else{'diagnostic-admin.manifest'}
        Run $compiler @('/nologo','/target:winexe','/platform:x64','/r:System.Windows.Forms.dll','/r:System.Security.dll','/r:System.Web.Extensions.dll','/r:Microsoft.CSharp.dll',"/out:$out\$exe","/win32manifest:$helper\$manifest","$helper\PrepareResume.cs","$helper\ResumeMaintenance.cs","$helper\AssemblyInfo.cs") "helper-build-$test.log"
    }
    # Source-only UI workspace. Copy tracked files, never an existing local build or credentials.
    $ui=Join-Path $repo 'tecassist-ui'
    if(Test-Path -LiteralPath $ui){throw 'tecassist-ui already exists; use a fresh checkout for each full build.'}
    New-Item -ItemType Directory -Path $ui | Out-Null
    foreach($rel in (& git ls-files)){
        $dest=Join-Path $ui $rel
        New-Item -ItemType Directory -Path (Split-Path $dest -Parent) -Force | Out-Null
        Copy-Item -LiteralPath (Join-Path $repo $rel) -Destination $dest -Force
    }
    # Upstream Windows CMake installs ../../target/release/librustdesk.dll.
    # Populate that path inside the isolated UI checkout with THIS build's DLL.
    New-Item -ItemType Directory -Path "$ui\target\release" -Force | Out-Null
    Copy-Item -LiteralPath "$out\librustdesk.dll" -Destination "$ui\target\release\librustdesk.dll" -Force
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    foreach($mode in @('console','standalone')){
        $overlay=Join-Path $PSScriptRoot "overlays\$mode\flutter"
        foreach($f in Get-ChildItem -LiteralPath $overlay -Recurse -File){
            $relative=$f.FullName.Substring($overlay.Length+1)
            $dest=Join-Path "$ui\flutter" $relative
            New-Item -ItemType Directory -Path (Split-Path $dest -Parent) -Force | Out-Null
            Copy-Item -LiteralPath $f.FullName -Destination $dest -Force
        }
        Set-Location "$ui\flutter"
        Run 'flutter' @('pub','get','--enforce-lockfile') "pub-$mode.log"
        Run 'flutter' @('build','windows','--release','--no-pub','--build-name=1.4.4','--build-number=25',"--dart-define=TEC_ASSIST_MODE=$mode") "flutter-$mode.log"
        $bundle=Join-Path $out "bundle-$mode"
        New-Item -ItemType Directory -Path $bundle | Out-Null
        Copy-Item "$ui\flutter\build\windows\x64\runner\Release\*" $bundle -Recurse
        $title=if($mode -eq 'console'){'Console'}else{'Standalone'}
        Move-Item "$bundle\rustdesk.exe" "$bundle\TecAssist_$title.exe"
        Copy-Item "$out\librustdesk.dll" "$bundle\librustdesk.dll" -Force
        Copy-Item "$repo\LICENCE" "$bundle\LICENZA_RUSTDESK.txt"
        if($mode -eq 'standalone'){Copy-Item "$out\TecAssist-PreparaRipresa.exe" $bundle}
        $dev=Join-Path $PSScriptRoot "packaging\$title"
        $payload=Join-Path $out "payload-$mode.zip"
        [IO.Compression.ZipFile]::CreateFromDirectory($bundle,$payload,[IO.Compression.CompressionLevel]::Optimal,$false)
        Run $compiler @('/nologo','/target:winexe','/platform:x64',"/win32icon:$dev\tecassist.ico",'/r:System.Windows.Forms.dll','/r:System.IO.Compression.dll','/r:System.IO.Compression.FileSystem.dll',"/resource:$payload,payload.zip","/out:$out\TecAssist-$title-1.4.4.25-Unsigned.exe","$dev\Launcher.cs") "launcher-$mode.log"
        $p=Start-Process -FilePath "$out\TecAssist-$title-1.4.4.25-Unsigned.exe" -ArgumentList '--verify-only' -Wait -PassThru -WindowStyle Hidden
        if($p.ExitCode -ne 0){throw "Launcher $title extraction check failed"}
    }
    $report=Join-Path $out 'native-selftest.json'
    $p=Start-Process -FilePath "$out\bundle-console\TecAssist_Console.exe" -ArgumentList "--tec-resume-selftest=$report" -Wait -PassThru -WindowStyle Hidden
    if(!(Test-Path -LiteralPath $report) -or !(Get-Content $report -Raw | ConvertFrom-Json).passed){throw 'Native selftest failed'}
    $report=Join-Path $out 'helper-selftest.json'
    $p=Start-Process -FilePath "$out\helper-test.exe" -ArgumentList @('--self-test',$report) -Wait -PassThru -WindowStyle Hidden
    if($p.ExitCode -ne 0 -or !(Get-Content $report -Raw | ConvertFrom-Json).passed){throw 'Helper selftest failed'}
    Get-ChildItem -LiteralPath $out -Filter '*.exe' | Get-FileHash -Algorithm SHA256 | Select-Object @{n='File';e={Split-Path $_.Path -Leaf}},Hash | ConvertTo-Json | Set-Content "$out\SHA256.json" -Encoding utf8
    @{version='1.4.4.25';sourceCommit=(& git -C $repo rev-parse HEAD);signed=$false;remoteConnectionTested=$false;websitePublished=$false;checksPassed=$true} | ConvertTo-Json | Set-Content "$out\build-status.json" -Encoding utf8
}finally{Pop-Location}
