@echo off
echo ==================================================
echo   Parallel String Theory OS
echo   One primitive. One loop. One OS.
echo ==================================================
echo.

set ISO=%~dp0bare-metal\tools\image\pst-os.iso

if not exist "%ISO%" (
    echo ERROR: ISO not found at %ISO%
    pause
    exit /b 1
)

:: Find VirtualBox or QEMU
set VBOX=
set QEMU=
where VBoxManage >nul 2>&1 && set VBOX=VBoxManage
if not defined VBOX if exist "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe" set "VBOX=C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
where qemu-system-x86_64 >nul 2>&1 && set QEMU=qemu-system-x86_64

if defined VBOX goto :vbox
if defined QEMU goto :qemu
echo No VM software found. Install VirtualBox or QEMU.
pause
exit /b 1

:vbox
echo Using VirtualBox
echo Killing any stale VBoxSVC...
taskkill /f /im VBoxSVC.exe >nul 2>&1
taskkill /f /im VBoxHeadless.exe >nul 2>&1
taskkill /f /im VirtualBoxVM.exe >nul 2>&1

echo Creating VM...
"%VBOX%" unregistervm "PST-OS" --delete >nul 2>&1
"%VBOX%" createvm --name "PST-OS" --ostype Other_64 --register
"%VBOX%" modifyvm "PST-OS" --memory 2048 --cpus 2 --graphicscontroller vboxvga --vram 64
"%VBOX%" modifyvm "PST-OS" --uart1 0x3F8 4 --uartmode1 file "%~dp0pst-serial.log"
"%VBOX%" storagectl "PST-OS" --name "IDE" --add ide
"%VBOX%" storageattach "PST-OS" --storagectl "IDE" --port 0 --device 0 --type dvddrive --medium "%ISO%"

echo.
echo Booting... Serial log: pst-serial.log
"%VBOX%" startvm "PST-OS"
pause
exit /b 0

:qemu
echo Using QEMU
echo.
"%QEMU%" -cdrom "%ISO%" -m 2G -serial stdio -no-reboot
pause
exit /b 0
