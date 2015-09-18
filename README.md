
Helper to find and bundle DLLs with your Windows binaries.

List dependencies:

    winbundle list file.exe
    ...

Or **bundle** everything into the same folder

    winbundle bundle dist/ file.exe
    ...

You might want to explicitly pass the sysroot path

    winbundle --sysroot /usr/x86_64-w64-mingw32/sys-root/mingw/ list file.exe

System DLLs (e.g. msvcrt.dll) should not be bundled (see SYSLIBS in src/main.rs).


