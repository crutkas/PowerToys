#include "pch.h"
#include <PowerRenameInterfaces.h>
#include <PowerRenameItem.h>
#include <shobjidl.h>
#include <iostream>

// Test function to demonstrate metadata extraction
void TestMetadataExtraction()
{
    // Create a PowerRenameItem for an image file
    CComPtr<IShellItem> spShellItem;
    // Use a sample image file path - replace with an actual path when testing
    HRESULT hr = SHCreateItemFromParsingName(L"C:\\test\\sample.jpg", nullptr, IID_PPV_ARGS(&spShellItem));
    if (SUCCEEDED(hr))
    {
        CComPtr<IPowerRenameItem> spRenameItem;
        hr = CPowerRenameItem::s_CreateInstance(spShellItem, IID_PPV_ARGS(&spRenameItem));
        if (SUCCEEDED(hr))
        {
            // Test file metadata
            PWSTR value = nullptr;
            if (SUCCEEDED(spRenameItem->GetFileProperty(L"System.Size", &value)) && value != nullptr)
            {
                std::wcout << L"File Size: " << value << std::endl;
                CoTaskMemFree(value);
            }
            
            if (SUCCEEDED(spRenameItem->GetFileProperty(L"System.DateCreated", &value)) && value != nullptr)
            {
                std::wcout << L"Date Created: " << value << std::endl;
                CoTaskMemFree(value);
            }
            
            // Test EXIF metadata
            if (SUCCEEDED(spRenameItem->GetImageProperty(L"System.Photo.CameraModel", &value)) && value != nullptr)
            {
                std::wcout << L"Camera Model: " << value << std::endl;
                CoTaskMemFree(value);
            }
            
            if (SUCCEEDED(spRenameItem->GetImageProperty(L"System.Photo.DateTaken", &value)) && value != nullptr)
            {
                std::wcout << L"Date Taken: " << value << std::endl;
                CoTaskMemFree(value);
            }
        }
    }
}