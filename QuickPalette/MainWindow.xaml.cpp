#include "pch.h"
#include "MainWindow.xaml.h"
#include <algorithm>
#include <cwchar>
#include <cwctype>
#include <filesystem>
#include <shlobj.h>
#include <shellapi.h>
#include <string_view>
#include <windows.h>
#include <winrt/Microsoft.UI.Xaml.Controls.h>
#include <winrt/Microsoft.UI.Xaml.Input.h>
#include <winrt/Windows.System.h>

using namespace winrt;
using namespace Microsoft::UI::Xaml;
using namespace Microsoft::UI::Xaml::Controls;

namespace
{
    constexpr size_t MaxVisibleApps = 8;

    std::wstring Normalize(std::wstring value)
    {
        std::transform(value.begin(), value.end(), value.begin(), [](wchar_t character)
        {
            return static_cast<wchar_t>(std::towlower(character));
        });
        return value;
    }

    void AddStartMenuApps(REFKNOWNFOLDERID folderId, std::vector<winrt::QuickPalette::implementation::AppEntry>& apps)
    {
        PWSTR folderPath = nullptr;
        const HRESULT result = SHGetKnownFolderPath(folderId, KF_FLAG_DEFAULT, nullptr, &folderPath);
        if (FAILED(result))
        {
            winrt::throw_hresult(result);
        }

        std::error_code error;
        const std::filesystem::path root(folderPath);
        CoTaskMemFree(folderPath);
        std::filesystem::recursive_directory_iterator iterator(
            root, std::filesystem::directory_options::skip_permission_denied, error);
        if (error)
        {
            winrt::throw_hresult(HRESULT_FROM_WIN32(error.value()));
        }
        const std::filesystem::recursive_directory_iterator end;

        for (; iterator != end; iterator.increment(error))
        {
            if (error)
            {
                error.clear();
                continue;
            }
            if (!iterator->is_regular_file(error) || error)
            {
                error.clear();
                continue;
            }

            const auto path = iterator->path();
            if (Normalize(path.extension().wstring()) != L".lnk")
            {
                continue;
            }

            const std::wstring name = path.stem().wstring();
            if (name.empty())
            {
                continue;
            }

            const auto normalizedName = Normalize(name);
            const bool duplicate = std::any_of(apps.begin(), apps.end(), [&](AppEntry const& app)
            {
                return app.normalizedName == normalizedName;
            });
            if (!duplicate)
            {
                apps.push_back({ name, normalizedName, path.wstring() });
            }
        }
    }

    std::wstring EncodeQuery(std::wstring_view query)
    {
        if (query.empty())
        {
            return {};
        }

        const int byteCount = WideCharToMultiByte(
            CP_UTF8, WC_ERR_INVALID_CHARS, query.data(), static_cast<int>(query.size()),
            nullptr, 0, nullptr, nullptr);
        if (byteCount <= 0)
        {
            winrt::throw_last_error();
        }

        std::string utf8(static_cast<size_t>(byteCount), '\0');
        if (WideCharToMultiByte(
                CP_UTF8, WC_ERR_INVALID_CHARS, query.data(), static_cast<int>(query.size()),
                utf8.data(), byteCount, nullptr, nullptr) != byteCount)
        {
            winrt::throw_last_error();
        }

        constexpr wchar_t Hex[] = L"0123456789ABCDEF";
        std::wstring encoded;
        encoded.reserve(utf8.size() * 3);
        for (unsigned char byte : utf8)
        {
            if ((byte >= 'a' && byte <= 'z') || (byte >= 'A' && byte <= 'Z') ||
                (byte >= '0' && byte <= '9') || byte == '-' || byte == '_' || byte == '.' || byte == '~')
            {
                encoded.push_back(static_cast<wchar_t>(byte));
            }
            else
            {
                encoded.push_back(L'%');
                encoded.push_back(Hex[byte >> 4]);
                encoded.push_back(Hex[byte & 0x0F]);
            }
        }
        return encoded;
    }
}

namespace winrt::QuickPalette::implementation
{
    MainWindow::MainWindow()
    {
        InitializeComponent();

        AddStartMenuApps(FOLDERID_Programs, m_apps);
        AddStartMenuApps(FOLDERID_CommonPrograms, m_apps);
        std::sort(m_apps.begin(), m_apps.end(), [](AppEntry const& left, AppEntry const& right)
        {
            return left.normalizedName < right.normalizedName;
        });
    }

    void MainWindow::InitializeWindow(HWND hwnd)
    {
        m_hwnd = hwnd;
    }

    void MainWindow::FocusSearch()
    {
        SearchBox().Text(L"");
        ShowWindow(m_hwnd, SW_SHOW);
        SearchBox().Focus(FocusState::Programmatic);
        RefreshResults();
    }

    void MainWindow::SearchBox_TextChanged(TextBox const& sender, TextChangedEventArgs const&)
    {
        m_query = sender.Text().c_str();
        RefreshResults();
    }

    void MainWindow::SearchBox_KeyDown(TextBox const&, Input::KeyRoutedEventArgs const& args)
    {
        if (args.Key() == Windows::System::VirtualKey::Escape)
        {
            ShowWindow(m_hwnd, SW_HIDE);
            SetWindowPos(m_hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            args.Handled(true);
        }
        else if (args.Key() == Windows::System::VirtualKey::Enter)
        {
            if (!m_visiblePaths.empty())
            {
                LaunchApp(m_visiblePaths.front());
            }
            else if (!m_query.empty())
            {
                SearchWeb(m_query);
            }
            args.Handled(true);
        }
    }

    void MainWindow::WebSearch_Click(Button const&, RoutedEventArgs const&)
    {
        SearchWeb(m_query);
    }

    void MainWindow::RefreshResults()
    {
        ResultsPanel().Children().Clear();
        m_visiblePaths.clear();

        const auto query = Normalize(m_query);
        if (query.empty())
        {
            StatusText().Text(L"Apps from your Start menu");
            WebSearchButton().Visibility(Visibility::Collapsed);
            return;
        }

        for (auto const& app : m_apps)
        {
            if (app.normalizedName.find(query) == std::wstring::npos)
            {
                continue;
            }

            auto button = Button();
            button.HorizontalContentAlignment(HorizontalAlignment::Left);
            button.Padding(Thickness{ 16, 10, 16, 10 });
            button.FontSize(16);
            button.Content(box_value(app.name));
            ResultsPanel().Children().Append(button);
            m_visiblePaths.push_back(app.path);

            const auto path = app.path;
            button.Click([this, path](IInspectable const&, RoutedEventArgs const&)
            {
                LaunchApp(path);
            });

            if (m_visiblePaths.size() == MaxVisibleApps)
            {
                break;
            }
        }

        StatusText().Text(m_visiblePaths.empty() ? L"No matching apps" : L"Apps");
        WebSearchButton().Visibility(Visibility::Visible);
    }

    void MainWindow::LaunchApp(std::wstring const& path)
    {
        ShowWindow(m_hwnd, SW_HIDE);
        SetWindowPos(m_hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);

        const auto result = reinterpret_cast<INT_PTR>(
            ShellExecuteW(nullptr, L"open", path.c_str(), nullptr, nullptr, SW_SHOWNORMAL));
        if (result <= 32)
        {
            wchar_t message[512]{};
            swprintf_s(message, L"Windows could not open this shortcut (error %Id).", result);
            MessageBoxW(m_hwnd, message, L"QuickPalette", MB_OK | MB_ICONERROR);
        }
    }

    void MainWindow::SearchWeb(std::wstring const& query)
    {
        const std::wstring url = L"https://www.google.com/search?q=" + EncodeQuery(query);
        const auto result = reinterpret_cast<INT_PTR>(
            ShellExecuteW(nullptr, L"open", url.c_str(), nullptr, nullptr, SW_SHOWNORMAL));
        if (result <= 32)
        {
            MessageBoxW(m_hwnd, L"Windows could not open your default browser.", L"QuickPalette", MB_OK | MB_ICONERROR);
        }
        else
        {
            ShowWindow(m_hwnd, SW_HIDE);
            SetWindowPos(m_hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
        }
    }
}
