#ifndef __FREETRACKCLIENT_FTTYPES_H__
#define __FREETRACKCLIENT_FTTYPES_H__

#include <inttypes.h>
#include <windows.h>

#define FT_PROGRAMID "FT_ProgramID"
#define FT_MM_DATA "FT_SharedMem"
#define FREETRACK "Freetrack"
#define FREETRACK_MUTEX "FT_Mutext"

typedef struct FreeTrackData
{
  uint32_t DataID;
  int32_t CamWidth;
  int32_t CamHeight;
  float Yaw, Pitch, Roll;
  float X, Y, Z;
  float RawYaw, RawPitch, RawRoll;
  float RawX, RawY, RawZ;
  float X1, Y1, X2, Y2, X3, Y3, X4, Y4;
} FreeTrackData;

typedef BOOL (WINAPI * FTGetData_t) (FreeTrackData *);
typedef LPCSTR (WINAPI * FTGetDllVersion_t) (VOID);
typedef LPCSTR (WINAPI * FTProvider_t) (VOID);
typedef VOID (WINAPI * FTReportName_t) (LPCSTR);

typedef struct FreeTrackProcs
{
  FTGetData_t FTGetData;
  FTGetDllVersion_t FTGetDllVersion;
  FTProvider_t FTProvider;
  FTReportName_t FTReportName;
} FreeTrackProcs;

static inline BOOL
FTLoadDll (FreeTrackProcs *Procs, HMODULE *FreeTrackClient)
{
  HKEY Key = NULL;
  LSTATUS Status;
  BYTE Path[1024];
  DWORD PathLen;
  CHAR FullPath[2048];

  if (!Procs || !FreeTrackClient)
    return FALSE;

  ZeroMemory (Procs, sizeof (FreeTrackProcs));

  Status = RegOpenKeyExA (HKEY_CURRENT_USER,
			  "Software\\Freetrack\\FreetrackClient", 0,
			  KEY_QUERY_VALUE, &Key);
  if (Status != ERROR_SUCCESS || !Key)
    return FALSE;

  PathLen = sizeof (Path);
  Status = RegQueryValueExA (Key, "Path", NULL, NULL, Path, &PathLen);
  RegCloseKey (Key);
  if (Status != ERROR_SUCCESS || PathLen == 0)
    return FALSE;

  wsprintfA (FullPath, "%s\\FreeTrackClient.dll", Path);

  *FreeTrackClient = LoadLibraryA (FullPath);
  if (!*FreeTrackClient)
    return FALSE;

  Procs->FTGetData =
    (FTGetData_t) GetProcAddress (*FreeTrackClient, "FTGetData");
  Procs->FTGetDllVersion =
    (FTGetDllVersion_t) GetProcAddress (*FreeTrackClient, "FTGetDllVersion");
  Procs->FTReportName =
    (FTReportName_t) GetProcAddress (*FreeTrackClient, "FTReportName");
  Procs->FTProvider =
    (FTProvider_t) GetProcAddress (*FreeTrackClient, "FTProvider");

  if (!Procs->FTGetData || !Procs->FTGetDllVersion || !Procs->FTReportName
      || !Procs->FTProvider)
    return FALSE;

  return TRUE;
}

#endif /* __FREETRACKCLIENT_FTTYPES_H__ */
