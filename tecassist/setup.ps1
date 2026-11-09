$ErrorActionPreference='Stop'
$repo=Split-Path $PSScriptRoot -Parent
$lock=Get-Content (Join-Path $PSScriptRoot 'source-lock.json') -Raw | ConvertFrom-Json
$deps=Join-Path $repo 'tecassist-deps'
New-Item -ItemType Directory -Path $deps -Force | Out-Null
if(!(Get-Command flutter -ErrorAction SilentlyContinue)){throw 'Install Flutter 3.24.5 and place it on PATH.'}
if(!(Get-Command rc.exe -ErrorAction SilentlyContinue)){throw 'Run in a Visual Studio 2022 x64 developer environment with Windows SDK.'}
& rustup toolchain install $lock.rust --profile minimal
if($LASTEXITCODE -ne 0){throw 'Rust setup failed'}
$env:RUSTUP_TOOLCHAIN=$lock.rust
if(!(Test-Path 'C:\Program Files\LLVM\bin\libclang.dll')){
    & choco install llvm --version=15.0.6 --yes --no-progress --allow-downgrade
    if($LASTEXITCODE -ne 0){throw 'LLVM install failed'}
}
$env:LIBCLANG_PATH='C:\Program Files\LLVM\bin'
$env:PATH="$env:LIBCLANG_PATH;$env:PATH"
$env:VCPKG_ROOT=Join-Path $deps 'vcpkg'
$env:VCPKG_INSTALLED_ROOT=Join-Path $env:VCPKG_ROOT 'installed'
$env:VCPKG_DEFAULT_TRIPLET='x64-windows-static'
$env:VCPKG_DEFAULT_HOST_TRIPLET='x64-windows-static'
if(!(Test-Path (Join-Path $env:VCPKG_ROOT '.git'))){
    & git clone https://github.com/microsoft/vcpkg.git $env:VCPKG_ROOT
    if($LASTEXITCODE -ne 0){throw 'vcpkg clone failed'}
}
& git -C $env:VCPKG_ROOT checkout --detach $lock.vcpkgCommit
if($LASTEXITCODE -ne 0){throw 'vcpkg revision checkout failed'}
& (Join-Path $env:VCPKG_ROOT 'bootstrap-vcpkg.bat') -disableMetrics
if($LASTEXITCODE -ne 0){throw 'vcpkg bootstrap failed'}
& (Join-Path $env:VCPKG_ROOT 'vcpkg.exe') install "--x-manifest-root=$repo" "--x-install-root=$env:VCPKG_INSTALLED_ROOT" --triplet=x64-windows-static
if($LASTEXITCODE -ne 0){throw 'vcpkg install failed'}
$env:CI='true'
& flutter precache --windows
if($LASTEXITCODE -ne 0){throw 'Flutter precache failed'}
$flutterBin=Split-Path (Get-Command flutter).Source -Parent
$flutterRoot=Split-Path $flutterBin -Parent
$engine=Join-Path $deps 'engine.zip'
Invoke-WebRequest -Uri $lock.engineUrl -OutFile $engine
if((Get-FileHash -LiteralPath $engine -Algorithm SHA256).Hash -ne $lock.engineSha256){throw 'Engine hash differs from pinned source-lock.json; do not bypass this check.'}
$engineStage=Join-Path $deps ('engine-'+[Guid]::NewGuid().ToString('N'))
Expand-Archive -LiteralPath $engine -DestinationPath $engineStage
Copy-Item "$engineStage\*" (Join-Path $flutterRoot 'bin\cache\artifacts\engine\windows-x64-release') -Force -Recurse
$patch=Join-Path $repo '.github\patches\flutter_3.24.4_dropdown_menu_enableFilter.diff'
& git -C $flutterRoot apply --check $patch 2>$null
if($LASTEXITCODE -eq 0){& git -C $flutterRoot apply $patch;if($LASTEXITCODE -ne 0){throw 'Flutter patch failed'}}else{
    & git -C $flutterRoot apply --reverse --check $patch
    if($LASTEXITCODE -ne 0){throw 'Flutter patch is neither applicable nor already applied'}
}
