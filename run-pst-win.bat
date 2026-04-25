@echo off
echo ==================================================
echo   Parallel String Theory OS
echo   One primitive. One loop. One OS.
echo ==================================================
echo.

set ISO=%~dp0bare-metal\tools\image\pst-os.iso

if not exist "%ISO%" (
    echo ERROR: ISO not found at %ISO%
    echo Build it first in WSL.
    pause
    exit /b 1
)

:: Try VirtualBox first (most likely already installed)
where VBoxManage >nul 2>&1
if %errorlevel%==0 (
    set VBOX=VBoxManage
    goto :virtualbox
)
if exist "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe" (
    set VBOX="C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
    goto :virtualbox
)

:: Fall back to QEMU
where qemu-system-x86_64 >nul 2>&1
if %errorlevel%==0 (
    set QEMU=qemu-system-x86_64
    goto :qemu
)
if exist "C:\Program Files\qemu\qemu-system-x86_64.exe" (
    set QEMU="C:\Program Files\qemu\qemu-system-x86_64.exe"
    goto :qemu
)

echo Neither VirtualBox nor QEMU found.
echo Install one of:
echo   VirtualBox: https://www.virtualbox.org/wiki/Downloads
echo   QEMU:       winget install SoftwareFreedomConservancy.QEMU
pause
exit /b 1

:virtualbox
echo Using VirtualBox
echo.

:: Clean up any previous PST OS VM
%VBOX% showvminfo "PST-OS" >nul 2>&1
if %errorlevel%==0 (
    %VBOX% controlvm "PST-OS" poweroff >nul 2>&1
    timeout /t 2 /nobreak >nul
    %VBOX% unregistervm "PST-OS" --delete >nul 2>&1
)

:: Create VM
%VBOX% createvm --name "PST-OS" --ostype Other_64 --register
%VBOX% modifyvm "PST-OS" --memory 2048 --cpus 2 --graphicscontroller vmsvga
%VBOX% modifyvm "PST-OS" --uart1 0x3F8 4 --uartmode1 file "%~dp0pst-serial.log"
%VBOX% storagectl "PST-OS" --name "IDE" --add ide
%VBOX% storageattach "PST-OS" --storagectl "IDE" --port 0 --device 0 --type dvddrive --medium "%ISO%"

echo.
echo Booting PST OS in VirtualBox...
echo Close the VM window to quit.
echo.
%VBOX% startvm "PST-OS"
goto :end

:qemu
echo Using QEMU
echo.
echo Booting PST OS... Close the window to quit.
echo.
%QEMU% -cdrom "%ISO%" -m 2G -serial stdio -no-reboot
goto :end

:end
