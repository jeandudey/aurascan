#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <windows.h>

#include "fttypes.h"

typedef BOOL (WINAPI *FTGetData_t)(FTData *);
typedef const char * (WINAPI *FTGetDllVersion_t)(void);
typedef const char * (WINAPI *FTProvider_t)(void);
typedef void (WINAPI *FTReportID_t)(int);
typedef void (WINAPI *FTReportName_t)(int);

#define PROC(dll, var, ty, name) \
  do { \
    if (!(var = (ty)GetProcAddress ((dll), (name)))) \
      { \
        printf ("Failed to resolve %s\n", (name)); \
        FreeLibrary ((dll)); \
        return 1; \
      } \
  } while (0)

int
main (void)
{
  HMODULE dll;
  FTGetData_t FTGetData;
  FTGetDllVersion_t FTGetDllVersion;
  FTProvider_t FTProvider;
  FTReportID_t FTReportID;
  FTReportName_t FTReportName;
  FTData data;
  const char *dll_version;
  const char *provider;

  dll = LoadLibraryA ("freetrackclient.dll");
  if (!dll)
    {
      printf ("Failed to load freetrackclient.dll\n");
      return 1;
    }

  PROC (dll, FTGetData, FTGetData_t, "FTGetData");
  PROC (dll, FTGetDllVersion, FTGetDllVersion_t, "FTGetDllVersion");
  PROC (dll, FTProvider, FTProvider_t, "FTProvider");
  PROC (dll, FTReportID, FTReportID_t, "FTReportID");
  PROC (dll, FTReportName, FTReportName_t, "FTReportName");

  FTReportID (0);
  FTReportName (0);

  if (!(dll_version = FTGetDllVersion ()))
    {
      printf ("FTGetDllVersion returned NULL\n");
      FreeLibrary (dll);
      return 1;
    }

  printf ("DLL version: %s\n", dll_version);
  if (strcmp (dll_version, "1.0.0.0") != 0)
    {
      printf ("Unexpected DLL version\n");
      return 1;
    }

  if (!(provider = FTProvider ()))
    {
      printf ("FTProvider returned NULL\n");
      FreeLibrary (dll);
      return 1;
    }

  printf ("Provider: %s\n", provider);
  if (strcmp (provider, "FreeTrack"))
    {
      printf ("Unexpected provider\n");
      return 1;
    }

  ZeroMemory (&data, sizeof (FTData));
  if (FTGetData (&data))
    printf("Yaw=%.2f Pitch=%.2f Roll=%.2f | X=%.2f Y=%.2f Z=%.2f\n",
            data.Yaw, data.Pitch, data.Roll,
            data.X, data.Y, data.Z);
  else
    {
      printf ("FTGetData failed / no data\n");
      FreeLibrary (dll);
      return 1;
    }

  FreeLibrary (dll);
  return 0;
}
