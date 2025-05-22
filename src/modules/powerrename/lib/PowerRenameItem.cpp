#include "pch.h"
#include "PowerRenameItem.h"
#include <propkey.h>
#include <propvarutil.h>
#include <strsafe.h>
#include <wincodec.h>

int CPowerRenameItem::s_id = 0;

IFACEMETHODIMP_(ULONG)
CPowerRenameItem::AddRef()
{
    return InterlockedIncrement(&m_refCount);
}

IFACEMETHODIMP_(ULONG)
CPowerRenameItem::Release()
{
    long refCount = InterlockedDecrement(&m_refCount);

    if (refCount == 0)
    {
        delete this;
    }
    return refCount;
}

IFACEMETHODIMP CPowerRenameItem::QueryInterface(_In_ REFIID riid, _Outptr_ void** ppv)
{
    static const QITAB qit[] = {
        QITABENT(CPowerRenameItem, IPowerRenameItem),
        QITABENT(CPowerRenameItem, IPowerRenameItemFactory),
        { 0 }
    };
    return QISearch(this, qit, riid, ppv);
}

IFACEMETHODIMP CPowerRenameItem::PutPath(_In_opt_ PCWSTR newPath)
{
    CSRWSharedAutoLock lock(&m_lock);
    CoTaskMemFree(m_path);
    m_path = nullptr;
    HRESULT hr = S_OK;
    if (newPath != nullptr)
    {
        hr = SHStrDup(newPath, &m_path);
    }
    return hr;
}

IFACEMETHODIMP CPowerRenameItem::GetPath(_Outptr_ PWSTR* path)
{
    *path = nullptr;
    CSRWSharedAutoLock lock(&m_lock);
    HRESULT hr = E_FAIL;
    if (m_path)
    {
        hr = SHStrDup(m_path, path);
    }
    return hr;
}

IFACEMETHODIMP CPowerRenameItem::GetTime(_Outptr_ SYSTEMTIME* time)
{
    CSRWSharedAutoLock lock(&m_lock);
    HRESULT hr = E_FAIL;

    if (m_isTimeParsed)
    {
        hr = S_OK;
    }
    else
    {
        HANDLE hFile = CreateFileW(m_path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS, NULL);
        if (hFile != INVALID_HANDLE_VALUE)
        {
            FILETIME CreationTime;
            if (GetFileTime(hFile, &CreationTime, NULL, NULL))
            {
                SYSTEMTIME SystemTime, LocalTime;
                if (FileTimeToSystemTime(&CreationTime, &SystemTime))
                {
                    if (SystemTimeToTzSpecificLocalTime(NULL, &SystemTime, &LocalTime))
                    {
                        m_time = LocalTime;
                        m_isTimeParsed = true;
                        hr = S_OK;
                    }
                }
            }
        }
        CloseHandle(hFile);
    }
    *time = m_time;
    return hr;
}

IFACEMETHODIMP CPowerRenameItem::GetShellItem(_Outptr_ IShellItem** ppsi)
{
    return SHCreateItemFromParsingName(m_path, nullptr, IID_PPV_ARGS(ppsi));
}

IFACEMETHODIMP CPowerRenameItem::PutOriginalName(_In_opt_ PCWSTR originalName)
{
    CSRWSharedAutoLock lock(&m_lock);
    CoTaskMemFree(m_originalName);
    m_originalName = nullptr;
    HRESULT hr = S_OK;
    if (originalName != nullptr)
    {
        hr = SHStrDup(originalName, &m_originalName);
    }
    return hr;
}

IFACEMETHODIMP CPowerRenameItem::GetOriginalName(_Outptr_ PWSTR* originalName)
{
    CSRWSharedAutoLock lock(&m_lock);
    HRESULT hr = E_FAIL;
    if (m_originalName)
    {
        hr = SHStrDup(m_originalName, originalName);
    }
    return hr;
}

IFACEMETHODIMP CPowerRenameItem::PutNewName(_In_opt_ PCWSTR newName)
{
    CSRWSharedAutoLock lock(&m_lock);
    CoTaskMemFree(m_newName);
    m_newName = nullptr;
    HRESULT hr = S_OK;
    if (newName != nullptr)
    {
        hr = SHStrDup(newName, &m_newName);
    }
    return hr;
}

IFACEMETHODIMP CPowerRenameItem::GetNewName(_Outptr_ PWSTR* newName)
{
    CSRWSharedAutoLock lock(&m_lock);
    HRESULT hr = S_OK;
    if (m_newName)
    {
        hr = SHStrDup(m_newName, newName);
    }
    return hr;
}

IFACEMETHODIMP CPowerRenameItem::GetIsFolder(_Out_ bool* isFolder)
{
    CSRWSharedAutoLock lock(&m_lock);
    *isFolder = m_isFolder;
    return S_OK;
}

IFACEMETHODIMP CPowerRenameItem::GetIsSubFolderContent(_Out_ bool* isSubFolderContent)
{
    CSRWSharedAutoLock lock(&m_lock);
    *isSubFolderContent = m_depth > 0;
    return S_OK;
}

IFACEMETHODIMP CPowerRenameItem::GetSelected(_Out_ bool* selected)
{
    CSRWSharedAutoLock lock(&m_lock);
    *selected = m_selected;
    return S_OK;
}

IFACEMETHODIMP CPowerRenameItem::PutSelected(_In_ bool selected)
{
    CSRWSharedAutoLock lock(&m_lock);
    m_selected = selected;
    return S_OK;
}

IFACEMETHODIMP CPowerRenameItem::GetId(_Out_ int* id)
{
    CSRWSharedAutoLock lock(&m_lock);
    *id = m_id;
    return S_OK;
}

IFACEMETHODIMP CPowerRenameItem::GetDepth(_Out_ UINT* depth)
{
    *depth = m_depth;
    return S_OK;
}

IFACEMETHODIMP CPowerRenameItem::PutDepth(_In_ int depth)
{
    m_depth = depth;
    return S_OK;
}

IFACEMETHODIMP CPowerRenameItem::GetStatus(_Out_ PowerRenameItemRenameStatus* status)
{
    *status = m_status;
    return S_OK;
}

IFACEMETHODIMP CPowerRenameItem::PutStatus(_In_ PowerRenameItemRenameStatus status)
{
    m_status = status;
    return S_OK;
}

IFACEMETHODIMP CPowerRenameItem::ShouldRenameItem(_In_ DWORD flags, _Out_ bool* shouldRename)
{
    // Should we perform a rename on this item given its
    // state and the options that were set?
    bool hasChanged = m_newName != nullptr && (lstrcmp(m_originalName, m_newName) != 0) && (lstrcmp(L"", m_newName) != 0);
    bool excludeBecauseFolder = (m_isFolder && (flags & PowerRenameFlags::ExcludeFolders));
    bool excludeBecauseFile = (!m_isFolder && (flags & PowerRenameFlags::ExcludeFiles));
    bool excludeBecauseSubFolderContent = (m_depth > 0 && (flags & PowerRenameFlags::ExcludeSubfolders));
    *shouldRename = (m_selected && m_canRename && hasChanged && !excludeBecauseFile &&
                     !excludeBecauseFolder && !excludeBecauseSubFolderContent && m_status == PowerRenameItemRenameStatus::ShouldRename);
    return S_OK;
}

IFACEMETHODIMP CPowerRenameItem::IsItemVisible(_In_ DWORD filter, _In_ DWORD flags, _Out_ bool* isItemVisible)
{
    switch (filter)
    {
    case PowerRenameFilters::None:
        *isItemVisible = true;
        break;
    case PowerRenameFilters::Selected:
        GetSelected(isItemVisible);
        break;
    case PowerRenameFilters::FlagsApplicable:
        *isItemVisible = !((m_isFolder && (flags & PowerRenameFlags::ExcludeFolders)) ||
                           (!m_isFolder && (flags & PowerRenameFlags::ExcludeFiles)) ||
                           (m_depth > 0 && (flags & PowerRenameFlags::ExcludeSubfolders)));
        break;
    case PowerRenameFilters::ShouldRename:
        ShouldRenameItem(flags, isItemVisible);
        break;
    }
    return S_OK;
}

IFACEMETHODIMP CPowerRenameItem::Reset()
{
    CSRWSharedAutoLock lock(&m_lock);
    CoTaskMemFree(m_newName);
    m_newName = nullptr;
    return S_OK;
}

IFACEMETHODIMP CPowerRenameItem::GetFileProperty(_In_ PCWSTR propertyName, _Outptr_ PWSTR* propertyValue)
{
    *propertyValue = nullptr;
    
    CSRWSharedAutoLock lock(&m_lock);
    HRESULT hr = E_FAIL;
    
    IShellItem2* psi2 = nullptr;
    IPropertyStore* pps = nullptr;
    PROPERTYKEY propKey;
    PROPVARIANT propVar;
    PropVariantInit(&propVar);
    
    // Common property names to property key mapping
    struct PropertyMapping {
        PCWSTR name;
        PROPERTYKEY key;
    };
    
    static const PropertyMapping propertyMappings[] = {
        { L"System.Size", PKEY_Size },
        { L"System.ItemType", PKEY_ItemType },
        { L"System.DateCreated", PKEY_DateCreated },
        { L"System.DateModified", PKEY_DateModified },
        { L"System.DateAccessed", PKEY_DateAccessed },
        { L"System.FileAttributes", PKEY_FileAttributes },
        { L"System.ComputerName", PKEY_ComputerName },
        { L"System.Author", PKEY_Author },
        { L"System.Title", PKEY_Title },
        { L"System.Subject", PKEY_Subject },
        { L"System.Keywords", PKEY_Keywords },
        { L"System.Comment", PKEY_Comment },
        { L"System.Copyright", PKEY_Copyright },
        // Music file properties
        { L"System.Music.AlbumTitle", PKEY_Music_AlbumTitle },
        { L"System.Music.Artist", PKEY_Music_Artist },
        { L"System.Music.Genre", PKEY_Music_Genre },
        // Video file properties
        { L"System.Video.FrameWidth", PKEY_Video_FrameWidth },
        { L"System.Video.FrameHeight", PKEY_Video_FrameHeight },
        { L"System.Video.FrameRate", PKEY_Video_FrameRate },
        // Image file properties
        { L"System.Image.Dimensions", PKEY_Image_Dimensions },
        { L"System.Image.HorizontalSize", PKEY_Image_HorizontalSize },
        { L"System.Image.VerticalSize", PKEY_Image_VerticalSize },
        { L"System.Image.BitDepth", PKEY_Image_BitDepth },
        // Document properties
        { L"System.Document.PageCount", PKEY_Document_PageCount }
    };
    
    // Convert property name to property key
    bool found = false;
    for (const auto& mapping : propertyMappings)
    {
        if (wcscmp(mapping.name, propertyName) == 0)
        {
            propKey = mapping.key;
            found = true;
            break;
        }
    }
    
    if (!found)
    {
        // If not found in our mapping, try to parse it
        hr = PSGetPropertyKeyFromName(propertyName, &propKey);
        if (FAILED(hr))
        {
            return hr;
        }
    }
    
    // Get the property from the shell item
    hr = SHCreateItemFromParsingName(m_path, nullptr, IID_PPV_ARGS(&psi2));
    if (SUCCEEDED(hr))
    {
        hr = psi2->GetPropertyStore(GPS_DEFAULT, IID_PPV_ARGS(&pps));
        if (SUCCEEDED(hr))
        {
            hr = pps->GetValue(propKey, &propVar);
            if (SUCCEEDED(hr))
            {
                PWSTR pszValue = nullptr;
                hr = PropVariantToStringAlloc(propVar, &pszValue);
                if (SUCCEEDED(hr) && pszValue != nullptr)
                {
                    *propertyValue = pszValue;
                }
            }
            
            pps->Release();
        }
        
        psi2->Release();
    }
    
    PropVariantClear(&propVar);
    
    return hr;
}

IFACEMETHODIMP CPowerRenameItem::GetImageProperty(_In_ PCWSTR propertyName, _Outptr_ PWSTR* propertyValue)
{
    *propertyValue = nullptr;
    
    CSRWSharedAutoLock lock(&m_lock);
    HRESULT hr = E_FAIL;
    
    // Check if file is an image first
    std::wstring ext = PathFindExtension(m_path);
    if (ext.empty())
    {
        return E_FAIL;
    }
    
    // Convert to lowercase for comparison
    for (auto& c : ext)
    {
        c = towlower(c);
    }
    
    // Check if this is an image file
    if (ext != L".jpg" && ext != L".jpeg" && ext != L".png" && ext != L".gif" && 
        ext != L".bmp" && ext != L".tiff" && ext != L".tif" && ext != L".heic")
    {
        return E_FAIL;
    }
    
    // EXIF property names to property key mapping
    struct ExifMapping {
        PCWSTR name;
        PCWSTR path;
    };
    
    static const ExifMapping exifMappings[] = {
        { L"System.Photo.ExposureTime", L"System.Photo.ExposureTime" },
        { L"System.Photo.FNumber", L"System.Photo.FNumber" },
        { L"System.Photo.ISOSpeed", L"System.Photo.ISOSpeed" },
        { L"System.Photo.ExposureBias", L"System.Photo.ExposureBias" },
        { L"System.Photo.FocalLength", L"System.Photo.FocalLength" },
        { L"System.Photo.Flash", L"System.Photo.Flash" },
        { L"System.Photo.Orientation", L"System.Photo.Orientation" },
        { L"System.Photo.MeteringMode", L"System.Photo.MeteringMode" },
        { L"System.Photo.LightSource", L"System.Photo.LightSource" },
        { L"System.Photo.DateTaken", L"System.Photo.DateTaken" },
        { L"System.Photo.CameraManufacturer", L"System.Photo.CameraManufacturer" },
        { L"System.Photo.CameraModel", L"System.Photo.CameraModel" },
        { L"System.Photo.FocalLengthInFilm", L"System.Photo.FocalLengthInFilm" },
        { L"System.Photo.DigitalZoom", L"System.Photo.DigitalZoom" },
        { L"System.GPS.Latitude", L"System.GPS.Latitude" },
        { L"System.GPS.Longitude", L"System.GPS.Longitude" },
        { L"System.GPS.Altitude", L"System.GPS.Altitude" },
    };
    
    // Find the EXIF property by name
    PCWSTR propertyPath = nullptr;
    for (const auto& mapping : exifMappings)
    {
        if (wcscmp(mapping.name, propertyName) == 0)
        {
            propertyPath = mapping.path;
            break;
        }
    }
    
    // If property is not in our mapping, use it directly
    if (propertyPath == nullptr)
    {
        propertyPath = propertyName;
    }
    
    // Get the EXIF property using property store
    IShellItem2* psi2 = nullptr;
    IPropertyStore* pps = nullptr;
    PROPERTYKEY propKey;
    PROPVARIANT propVar;
    PropVariantInit(&propVar);
    
    hr = PSGetPropertyKeyFromName(propertyPath, &propKey);
    if (SUCCEEDED(hr))
    {
        hr = SHCreateItemFromParsingName(m_path, nullptr, IID_PPV_ARGS(&psi2));
        if (SUCCEEDED(hr))
        {
            hr = psi2->GetPropertyStore(GPS_DEFAULT, IID_PPV_ARGS(&pps));
            if (SUCCEEDED(hr))
            {
                hr = pps->GetValue(propKey, &propVar);
                if (SUCCEEDED(hr))
                {
                    PWSTR pszValue = nullptr;
                    hr = PropVariantToStringAlloc(propVar, &pszValue);
                    if (SUCCEEDED(hr) && pszValue != nullptr)
                    {
                        *propertyValue = pszValue;
                    }
                }
                
                pps->Release();
            }
            
            psi2->Release();
        }
    }
    
    PropVariantClear(&propVar);
    
    return hr;
}

HRESULT CPowerRenameItem::s_CreateInstance(_In_opt_ IShellItem* psi, _In_ REFIID iid, _Outptr_ void** resultInterface)
{
    *resultInterface = nullptr;

    CPowerRenameItem* newRenameItem = new CPowerRenameItem();
    HRESULT hr = E_OUTOFMEMORY;
    if (newRenameItem)
    {
        hr = S_OK;
        if (psi != nullptr)
        {
            hr = newRenameItem->_Init(psi);
        }

        if (SUCCEEDED(hr))
        {
            hr = newRenameItem->QueryInterface(iid, resultInterface);
        }

        newRenameItem->Release();
    }
    return hr;
}

CPowerRenameItem::CPowerRenameItem() :
    m_refCount(1),
    m_id(++s_id)
{
}

CPowerRenameItem::~CPowerRenameItem()
{
    CoTaskMemFree(m_path);
    CoTaskMemFree(m_newName);
    CoTaskMemFree(m_originalName);
}

HRESULT CPowerRenameItem::_Init(_In_ IShellItem* psi)
{
    // Get the full filesystem path from the shell item
    HRESULT hr = psi->GetDisplayName(SIGDN_FILESYSPATH, &m_path);
    if (SUCCEEDED(hr))
    {
        hr = SHStrDup(PathFindFileName(m_path), &m_originalName);
        if (SUCCEEDED(hr))
        {
            // Check if we are a folder now so we can check this attribute quickly later
            // Also check if the shell allows us to rename the item.
            SFGAOF att = 0;
            hr = psi->GetAttributes(SFGAO_STREAM | SFGAO_FOLDER | SFGAO_CANRENAME, &att);
            if (SUCCEEDED(hr))
            {
                // Some items can be both folders and streams (ex: zip folders).
                m_isFolder = (att & SFGAO_FOLDER) && !(att & SFGAO_STREAM);
                // The shell lets us know if an item should not be renamed
                // (ex: user profile director, windows dir, etc).
                m_canRename = (att & SFGAO_CANRENAME);
            }
        }
    }

    return hr;
}
