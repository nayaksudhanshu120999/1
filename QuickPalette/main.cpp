#include "pch.h"
#include "App.xaml.h"
#include <winrt/Microsoft.UI.Xaml.h>

int __stdcall wWinMain(HINSTANCE, HINSTANCE, PWSTR, int)
{
    winrt::init_apartment(winrt::apartment_type::single_threaded);
    Microsoft::UI::Xaml::Application::Start([](auto&&)
    {
        winrt::make<winrt::QuickPalette::implementation::App>();
    });
}
