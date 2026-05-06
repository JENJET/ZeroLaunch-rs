@echo off
chcp 65001 >nul

echo [信息] ZeroLaunch-rs 快速构建 - 构建所有版本
echo [信息] 开始时间: %date% %time%
echo.

if not exist "xtask\Cargo.toml" (
    echo [错误] 请在项目根目录下运行此脚本！
    pause
    exit /b 1
)

:: 切换到 xtask 目录
cd xtask

cargo run --bin xtask build-all

if %errorlevel% equ 0 (
    echo.
    echo [成功] 构建完成！
    echo [信息] 结束时间: %date% %time%
) else (
    echo.
    echo [错误] 构建失败！
    exit /b 1
)

pause
