#pragma once

#include "MainWindow.xaml.g.h"
#include <windows.h>
#include <string>
#include <vector>
#include <winrt/Microsoft.UI.Xaml.Controls.h>
#include <winrt/Microsoft.UI.Xaml.Input.h>

namespace winrt::QuickPalette::implementation
{
    struct AppEntry
    {
        std::wstring name;
        std::wstring normalizedName;
        std::wstring path;
    };

    struct MainWindow : MainWindowT<MainWindow>
    {
        MainWindow();

        void InitializeWindow(HWND hwnd);
        void FocusSearch();

        void SearchBox_TextChanged(
            Microsoft::UI::Xaml::Controls::TextBox const& sender,
            Microsoft::UI::Xaml::Controls::TextChangedEventArgs const& args);
        void SearchBox_KeyDown(
            Microsoft::UI::Xaml::Controls::TextBox const& sender,
            Microsoft::UI::Xaml::Input::KeyRoutedEventArgs const& args);
        void WebSearch_Click(
            Microsoft::UI::Xaml::Controls::Button const& sender,
            Microsoft::UI::Xaml::RoutedEventArgs const& args);

    private:
        void RefreshResults();
        void LaunchApp(std::wstring const& path);
        void SearchWeb(std::wstring const& query);

        HWND m_hwnd{ nullptr };
        std::vector<AppEntry> m_apps;
        std::vector<std::wstring> m_visiblePaths;
        std::wstring m_query;
    };
}

namespace winrt::QuickPalette::factory_implementation
{
    struct MainWindow : MainWindowT<MainWindow, implementation::MainWindow>
    {
    };
}
