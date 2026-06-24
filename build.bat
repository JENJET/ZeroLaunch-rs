@echo off
setlocal enabledelayedexpansion

echo.
echo ============================================================
echo        ZeroLaunch-rs 一键打包工具
echo        使用 xtask 构建所有版本
echo ============================================================
echo.

if not exist "xtask\Cargo.toml" (
    echo [错误] 请在项目根目录下运行此脚本！
    pause
    exit /b 1
)

set "VS_INSTALL_PATH="
if exist "C:\Program Files\Microsoft Visual Studio\18\Community" set "VS_INSTALL_PATH=C:\Program Files\Microsoft Visual Studio\18\Community"
if exist "C:\Program Files\Microsoft Visual Studio\17\Community" set "VS_INSTALL_PATH=C:\Program Files\Microsoft Visual Studio\17\Community"
if exist "C:\Program Files\Microsoft Visual Studio\16\Community" set "VS_INSTALL_PATH=C:\Program Files\Microsoft Visual Studio\16\Community"

if "%VS_INSTALL_PATH%"=="" (
    echo [错误] 未找到 Visual Studio 安装，请先安装 Visual Studio 2022+
    pause
    exit /b 1
)

set "VCVARSALL=%VS_INSTALL_PATH%\VC\Auxiliary\Build\vcvarsall.bat"
if not exist "%VCVARSALL%" (
    echo [错误] 未找到 vcvarsall.bat，请检查 Visual Studio 安装
    pause
    exit /b 1
)

set "HAS_ARM64_TOOLS=0"
for /f "delims=" %%i in ('"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -requires Microsoft.VisualStudio.Component.VC.Tools.ARM64') do set "HAS_ARM64_TOOLS=1"

echo [信息] 开始时间: %date% %time%
echo.

echo 请选择构建模式:
echo   1. 构建所有版本 (x64 + ARM64, 启用AI + 禁用AI) [推荐]
echo   2. 仅构建启用AI版本 (x64 + ARM64)
echo   3. 仅构建禁用AI版本 (x64 + ARM64)
echo   4. 仅构建 x64 架构 (启用AI + 禁用AI)
echo   5. 仅构建 ARM64 架构 (启用AI + 禁用AI)
echo   6. 快速构建: 仅 x64 便携版 + 启用AI版本
echo   7. 清理构建产物
echo.
set /p choice="请输入选项 (1-7, 默认 1): "

if "%choice%"=="" set choice=1

echo.
echo [信息] 您选择了选项: %choice%
echo.

if "%choice%"=="1" goto :check_arm64
if "%choice%"=="2" goto :check_arm64
if "%choice%"=="3" goto :check_arm64
if "%choice%"=="5" goto :check_arm64
goto :run

:check_arm64
if "%HAS_ARM64_TOOLS%"=="0" (
    echo [错误] 缺少 ARM64 交叉编译工具！
    echo.
    echo 请以管理员身份运行以下命令安装:
    echo   "C:\Program Files (x86)\Microsoft Visual Studio\Installer\setup.exe" ^
    echo        modify --installPath "%VS_INSTALL_PATH%" ^
    echo        --add Microsoft.VisualStudio.Component.VC.Tools.ARM64 ^
    echo        --passive --norestart
    echo.
    echo 安装完成后，重新运行此脚本。
    echo.
    pause
    exit /b 1
)

if "%choice%"=="7" goto :run
call "%VCVARSALL%" amd64 >nul 2>nul
if %errorlevel% neq 0 (
    echo [错误] Visual Studio 编译环境初始化失败！请确保已安装了"使用C++的桌面开发"工作负载
    pause
    exit /b 1
)

:run
cd xtask

if "%choice%"=="1" (
    cargo run --bin xtask clean
    cargo run --bin xtask build-all
) else if "%choice%"=="2" (
    cargo run --bin xtask build-all --ai enabled
) else if "%choice%"=="3" (
    cargo run --bin xtask build-all --ai disabled
) else if "%choice%"=="4" (
    cargo run --bin xtask build-all --arch x64
) else if "%choice%"=="5" (
    cargo run --bin xtask build-all --arch arm64
) else if "%choice%"=="6" (
    cargo run --bin xtask build-portable --arch x64 --ai enabled
) else if "%choice%"=="7" (
    cargo run --bin xtask clean
    goto :end
) else (
    echo [错误] 无效的选项！
    pause
    exit /b 1
)

if %errorlevel% equ 0 (
    echo.
    echo ============================================================
    echo                    构建成功！
    echo ============================================================
) else (
    echo.
    echo ============================================================
    echo                    构建失败！
    echo ============================================================
)

:end
echo.
echo [信息] 结束时间: %date% %time%
echo.
pause
