# QuickPalette

QuickPalette is a small C++/WinUI 3 launcher for Windows. Press **Ctrl+Space**
to open the centered search palette, type an app name to find a Start menu
shortcut, or choose **Search the web** to search in your default browser.
Press **Enter** to launch the first matching app (or search the web if there
are no app matches), and **Esc** to hide the palette. The app stays in the
notification area so the global shortcut remains available; choose **Exit**
from its tray menu to close it.

The launcher indexes Start menu shortcuts once at startup and filters that
small in-memory list as you type. While hidden it waits for Windows messages
and does no polling, so idle CPU use is negligible. A global hotkey does require
a resident process and therefore cannot use literally zero memory.

## Get a Windows build without a Windows PC

The source is a Visual Studio C++/WinUI 3 project. This Linux workspace does
not include the Windows SDK, MSVC, or Windows App SDK, so it cannot compile the
executable locally. The included GitHub Actions workflow builds it on a
Microsoft-hosted Windows runner:

1. Push this repository to GitHub.
2. In **Actions**, run **Build QuickPalette** (or push to the `main` branch).
3. Download the **QuickPalette-windows-x64** workflow artifact.
4. Extract the ZIP and run `QuickPalette.exe`.

The build is unpackaged and self-contained, so the extracted output directory
must stay together; it is not a single-file executable. The first launch may
be blocked by Windows SmartScreen because the app is not code-signed. If
`Ctrl+Space` is already reserved by Windows or another app, choose a different
shortcut before running the app (the current version reports the conflict and
exits).

To build in Visual Studio, install **Visual Studio 2022** with the **Desktop
development with C++** workload and a Windows 10/11 SDK, then open
`QuickPalette.sln` and build the **Release / x64** configuration.
