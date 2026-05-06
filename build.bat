@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion

echo.
echo ============================================================
echo        ZeroLaunch-rs 一键打包工具
echo        使用 xtask 构建所有版本
echo ============================================================
echo.

:: 检查是否在项目根目录
if not exist "xtask\Cargo.toml" (
    echo [错误] 请在项目根目录下运行此脚本！
    pause
    exit /b 1
)

:: 切换到父目录（xtask 需要在其子目录下运行）
cd xtask

:: 显示当前时间
echo [信息] 开始时间: %date% %time%
echo.

:: 询问用户选择构建模式
echo 请选择构建模式:
echo   1. 构建所有版本 (x64 + ARM64, 启用AI + 禁用AI) [推荐]
echo   2. 仅构建启用AI版本 (x64 + ARM64)
echo   3. 仅构建禁用AI版本 (x64 + ARM64)
echo   4. 仅构建 x64 架构 (启用AI + 禁用AI)
echo   5. 仅构建 ARM64 架构 (启用AI + 禁用AI)
echo   6. 快速构建: 仅 x64 启用AI版本
echo   7. 清理构建产物
echo.
set /p choice="请输入选项 (1-7, 默认 1): "

if "%choice%"=="" set choice=1

echo.
echo [信息] 您选择了选项: %choice%
echo.

:: 根据选择执行不同的构建命令
if "%choice%"=="1" (
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
    cargo run --bin xtask build-installer --arch x64 --ai enabled
) else if "%choice%"=="7" (
    cargo run --bin xtask clean
    goto :end
) else (
    echo [错误] 无效的选项！
    pause
    exit /b 1
)

:: 检查构建结果
if %errorlevel% equ 0 (
    echo.
    echo ============================================================
    echo                      构建成功！
    echo ============================================================
) else (
    echo.
    echo ============================================================
    echo                      构建失败！
    echo ============================================================
)

:end
echo.
echo [信息] 结束时间: %date% %time%
echo.
pause
