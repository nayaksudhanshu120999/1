#include "pch.h"
#include "App.xaml.h"
#include "MainWindow.xaml.h"
#include <winrt/Microsoft.UI.Xaml.h>
#include <winrt/Microsoft.UI.Xaml.Controls.h>
#include <winrt/Microsoft.UI.Xaml.Controls.Primitives.h>
#include <winrt/Microsoft.UI.Xaml.Data.h>
#include <winrt/Microsoft.UI.Xaml.Interop.h>
#include <winrt/Microsoft.UI.Xaml.Markup.h>
#include <winrt/Microsoft.UI.Xaml.Media.h>
#include <winrt/Microsoft.UI.Xaml.Navigation.h>
#include <microsoft.ui.xaml.window.h>
#include <shellapi.h>
#include <cwchar>

using namespace winrt;
using namespace Microsoft::UI::Xaml;

namespace
{
    constexpr int PaletteHotkeyId = 1;
    constexpr UINT TrayCallbackMessage = WM_APP + 1;
    constexpr UINT ExitMenuCommand = 1001;
}

namespace winrt::QuickPalette::implementation
{
    App::App()
    {
        InitializeComponent();
    }

    App::~App()
    {
        if (m_trayIconAdded)
        {
            Shell_NotifyIconW(NIM_DELETE, &m_trayIcon);
        }
        if (m_hotkeyRegistered)
        {
            UnregisterHotKey(m_hwnd, PaletteHotkeyId);
        }
        if (m_hwnd)
        {
            RemoveWindowSubclass(m_hwnd, WindowSubclassProc, PaletteHotkeyId);
        }
    }

    void App::OnLaunched(LaunchActivatedEventArgs const&)
    {
        auto palette = winrt::make<winrt::QuickPalette::implementation::MainWindow>();
        m_window = palette;
        m_window.Activate();

        auto nativeWindow = m_window.as<IWindowNative>();
        winrt::check_hresult(nativeWindow->get_WindowHandle(&m_hwnd));
        winrt::get_self<winrt::QuickPalette::implementation::MainWindow>(m_window)->InitializeWindow(m_hwnd);

        const int width = 760;
        const int height = 500;
        const int left = (GetSystemMetrics(SM_CXSCREEN) - width) / 2;
        const int top = (GetSystemMetrics(SM_CYSCREEN) - height) / 2;
        SetWindowPos(m_hwnd, nullptr, left, top, width, height, SWP_NOZORDER | SWP_NOACTIVATE);

        if (!SetWindowSubclass(m_hwnd, WindowSubclassProc, PaletteHotkeyId, reinterpret_cast<DWORD_PTR>(this)))
        {
            winrt::throw_last_error();
        }

        if (!RegisterHotKey(m_hwnd, PaletteHotkeyId, MOD_CONTROL | MOD_NOREPEAT, VK_SPACE))
        {
            const DWORD error = GetLastError();
            wchar_t message[256]{};
            swprintf_s(message, L"QuickPalette could not register Ctrl+Space (Windows error %lu). Close the app that uses this shortcut and try again.", error);
            MessageBoxW(m_hwnd, message, L"QuickPalette", MB_OK | MB_ICONERROR);
            Exit();
            return;
        }
        m_hotkeyRegistered = true;

        m_trayIcon.cbSize = sizeof(m_trayIcon);
        m_trayIcon.hWnd = m_hwnd;
        m_trayIcon.uID = PaletteHotkeyId;
        m_trayIcon.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        m_trayIcon.uCallbackMessage = TrayCallbackMessage;
        m_trayIcon.hIcon = LoadIconW(nullptr, IDI_APPLICATION);
        wcscpy_s(m_trayIcon.szTip, L"QuickPalette - Ctrl+Space");
        m_trayIconAdded = Shell_NotifyIconW(NIM_ADD, &m_trayIcon) == TRUE;
        if (!m_trayIconAdded)
        {
            MessageBoxW(m_hwnd, L"QuickPalette could not create its notification-area icon.", L"QuickPalette", MB_OK | MB_ICONERROR);
            Exit();
            return;
        }

        ShowWindow(m_hwnd, SW_HIDE);
    }

    LRESULT CALLBACK App::WindowSubclassProc(
        HWND hwnd,
        UINT message,
        WPARAM wParam,
        LPARAM lParam,
        UINT_PTR subclassId,
        DWORD_PTR refData)
    {
        auto self = reinterpret_cast<App*>(refData);

        if (message == WM_HOTKEY && wParam == PaletteHotkeyId)
        {
            self->ShowPalette();
            return 0;
        }
        if (message == TrayCallbackMessage)
        {
            if (LOWORD(lParam) == WM_LBUTTONUP || LOWORD(lParam) == WM_LBUTTONDBLCLK)
            {
                self->ShowPalette();
                return 0;
            }
            if (LOWORD(lParam) == WM_RBUTTONUP || LOWORD(lParam) == WM_CONTEXTMENU)
            {
                self->ShowTrayMenu();
                return 0;
            }
        }
        if (message == WM_COMMAND && LOWORD(wParam) == ExitMenuCommand)
        {
            self->Exit();
            return 0;
        }
        if (message == WM_NCDESTROY)
        {
            RemoveWindowSubclass(hwnd, WindowSubclassProc, subclassId);
        }

        return DefSubclassProc(hwnd, message, wParam, lParam);
    }

    void App::ShowPalette()
    {
        ShowWindow(m_hwnd, SW_RESTORE);
        SetWindowPos(m_hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
        SetForegroundWindow(m_hwnd);
        auto palette = winrt::get_self<winrt::QuickPalette::implementation::MainWindow>(m_window);
        palette->FocusSearch();
    }

    void App::ShowTrayMenu()
    {
        POINT cursor{};
        GetCursorPos(&cursor);

        HMENU menu = CreatePopupMenu();
        if (!menu)
        {
            winrt::throw_last_error();
        }
        AppendMenuW(menu, MF_STRING, ExitMenuCommand, L"Exit QuickPalette");
        SetForegroundWindow(m_hwnd);
        TrackPopupMenu(menu, TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, cursor.x, cursor.y, 0, m_hwnd, nullptr);
        DestroyMenu(menu);
    }

    void App::Exit()
    {
        if (m_trayIconAdded)
        {
            Shell_NotifyIconW(NIM_DELETE, &m_trayIcon);
            m_trayIconAdded = false;
        }
        if (m_hotkeyRegistered)
        {
            UnregisterHotKey(m_hwnd, PaletteHotkeyId);
            m_hotkeyRegistered = false;
        }
        m_window.Close();
    }
}
