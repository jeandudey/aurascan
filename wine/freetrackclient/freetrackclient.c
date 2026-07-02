#include <windows.h>

#include "freetrackclient.h"

static HANDLE FTMemMap = NULL;
static FreeTrackData *FTData = NULL;
static HANDLE *FTHandle = NULL;
static HANDLE FTMutex = NULL;
static LPSTR FTProgramName = NULL;
static uint32_t LastDataID = 0xFFFFFFFF;
static CHAR FTDllVersion[24];

static VOID
DestroyMapping (VOID)
{
  if (FTData)
    {
      UnmapViewOfFile (FTData);
      FTData = NULL;
      FTHandle = NULL;
      FTProgramName = NULL;
    }

  if (FTMemMap)
    {
      CloseHandle (FTMemMap);
      FTMemMap = NULL;
    }

  if (FTMutex)
    {
      CloseHandle (FTMutex);
      FTMutex = NULL;
    }
}

static BOOL
OpenMapping (VOID)
{
  SIZE_T MapLen;
  BOOL ret;

  if (FTMemMap)
    return TRUE;


  MapLen = sizeof (FreeTrackData) + sizeof (HANDLE) + 100;
  FTMemMap = CreateFileMappingA (INVALID_HANDLE_VALUE, NULL, PAGE_READWRITE,
                                 0, MapLen, FT_MM_DATA);
  if (!FTMemMap)
    return FALSE;

  FTData = MapViewOfFile (FTMemMap, FILE_MAP_WRITE, 0, 0, MapLen);
  if (!FTData)
    {
      ret = FALSE;
      goto exit;
    }

  FTHandle = (HANDLE *) ((BYTE *) FTData + sizeof (FreeTrackData));
  FTProgramName = (char *) ((BYTE *) FTHandle + sizeof (HANDLE));

  FTMutex = CreateMutexA (NULL, FALSE, FREETRACK_MUTEX);
  if (!FTMutex)
    {
      ret = FALSE;
      goto exit;
    }

exit:
  if (ret == FALSE)
    DestroyMapping ();
  return ret;
}


BOOL WINAPI
FTGetData (FreeTrackData *Data)
{
  if (!Data)
    return FALSE;

  if (!OpenMapping ())
    return FALSE;

  if (WaitForSingleObject (FTMutex, 0) != WAIT_OBJECT_0)
    return FALSE;

  if (FTData->DataID != LastDataID)
    {
      CopyMemory (Data, FTData, sizeof (FreeTrackData));
      LastDataID = Data->DataID;
      if (LastDataID > (1 << 29))
        FTData->DataID = 0;
      ReleaseMutex (FTMutex);
      return TRUE;
    }

  ReleaseMutex (FTMutex);
  return FALSE;
}

VOID WINAPI
FTReportName (LPCSTR Name)
{
  int Len;
  UINT MsgID;

  if (!Name)
    return;

  if (!OpenMapping ())
    return;

  if (WaitForSingleObject (FTMutex, INFINITE) != WAIT_OBJECT_0)
    return;

  Len = lstrlenA (Name);
  if (Len > 99)
    Len = 99;

  CopyMemory (FTProgramName, Name, Len);
  FTProgramName[Len] = 0;

  MsgID = RegisterWindowMessageA (FT_PROGRAMID);
  SendMessageTimeoutA (*FTHandle, MsgID, 0, 0, 0, 2000, NULL);

  ReleaseMutex (FTMutex);
}

VOID WINAPI
FTReportID (int Name)
{
  (void) Name;
}

LPCSTR WINAPI
FTGetDllVersion (VOID)
{
  HMODULE Module;
  DWORD Flags;
  CHAR FileName[MAX_PATH];
  DWORD Dummy, VersionInfoSize;
  PVOID VersionInfo;
  HANDLE Heap;
  VS_FIXEDFILEINFO *FixedFileInfo;
  UINT Len;
  WORD Major, Minor, Build, Rev;

  Flags =
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS |
    GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
  if (!GetModuleHandleExA (Flags, (LPCSTR) & FTGetDllVersion, &Module))
    return NULL;

  GetModuleFileNameA (Module, FileName, MAX_PATH);

  VersionInfoSize = GetFileVersionInfoSizeA (FileName, &Dummy);
  if (VersionInfoSize == 0)
    return NULL;

  Heap = GetProcessHeap ();
  if (!Heap)
    return NULL;

  VersionInfo = HeapAlloc (Heap, 0, VersionInfoSize);
  if (!GetFileVersionInfoA (FileName, 0, VersionInfoSize, VersionInfo))
    {
      HeapFree (Heap, 0, VersionInfo);
      return NULL;
    }

  if (!VerQueryValueA (VersionInfo, "\\", (LPVOID *) & FixedFileInfo, &Len))
    {
      HeapFree (Heap, 0, VersionInfo);
      return NULL;
    }

  Major = HIWORD (FixedFileInfo->dwFileVersionMS);
  Minor = LOWORD (FixedFileInfo->dwFileVersionMS);
  Build = HIWORD (FixedFileInfo->dwFileVersionLS);
  Rev = LOWORD (FixedFileInfo->dwFileVersionLS);
  HeapFree (Heap, 0, VersionInfo);

  wsprintfA (FTDllVersion, "%d.%d.%d.%d", Major, Minor, Build, Rev);
  return FTDllVersion;
}

LPCSTR WINAPI
FTProvider (VOID)
{
  return FREETRACK;
}

BOOL WINAPI
DllMain (HINSTANCE instance, DWORD reason)
{
  switch (reason)
    {
    case DLL_PROCESS_ATTACH:
      OpenMapping ();
      break;
    case DLL_PROCESS_DETACH:
      DestroyMapping ();
      break;
    }
  return TRUE;
}
