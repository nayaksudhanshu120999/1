#pragma once

#include "App.xaml.g.h"
#include <windows.h>
#include <commctrl.h>
#include <shellapi.h>

namespace winrt::QuickPalette::implementation
{
    struct App : AppT<App>
    {
        App();
        void OnLaunched(Microsoft::UI::Xaml::LaunchActivatedEventArgs const& args);
        ~App();

    private:
        static LRESULT CALLBACK WindowSubclassProc(
            HWND hwnd,
            UINT message,
            WPARAM wParam,
            LPARAM lParam,
            UINT_PTR subclassId,
            DWORD_PTR refData);

        void ShowPalette();
        void ShowTrayMenu();
        void Exit();

        Microsoft::UI::Xaml::Window m_window{ nullptr };
        HWND m_hwnd{ nullptr };
        NOTIFYICONDATAW m_trayIcon{};
        bool m_hotkeyRegistered{ false };
        bool m_trayIconAdded{ false };
    };
}

namespace winrt::QuickPalette::factory_implementation
{
    struct App : AppT<App, implementation::App>
    {
    };
}
