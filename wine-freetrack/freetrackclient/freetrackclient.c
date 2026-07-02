#include <stdarg.h>

#include <windef.h>
#include <winbase.h>
#include <ntstatus.h>

#include <wine/unixlib.h>
#include <wine/debug.h>

#include "fttypes.h"
#include "unixlib.h"

WINE_DEFAULT_DEBUG_CHANNEL(freetrack);

BOOL WINAPI
FTGetData (FTData *data)
{
  NTSTATUS status;
  struct FTGetData_params args;

  if (!data)
    return FALSE;

  args.data = data;
  status = WINE_UNIX_CALL (unix_FTGetData, &args);
  switch (status)
    {
    case STATUS_SUCCESS:
      return TRUE;
    case STATUS_CANT_WAIT:
      ZeroMemory (&data, sizeof (FTData));
      return TRUE;
    default:
      WINE_ERR ("unix call failed, status %lx.\n", (unsigned long)status);
      break;
    }

  return FALSE;
}

const char * WINAPI
FTGetDllVersion (void)
{
  return "1.0.0.0";
}

const char * WINAPI
FTProvider (void)
{
  return "FreeTrack";
}

void WINAPI
FTReportName (int name)
{
  WINE_FIXME ("(%i) stub\n", name);
}

void WINAPI
FTReportID (int name)
{
  WINE_FIXME ("(%i) stub\n", name);
}

BOOL WINAPI
DllMain (HINSTANCE instance, DWORD reason, LPVOID reserved)
{
  NTSTATUS status;

  switch (reason)
    {
    case DLL_PROCESS_ATTACH:
      DisableThreadLibraryCalls (instance);
      if ((status = __wine_init_unix_call()))
        {
          WINE_ERR ("__wine_init_unix_call failed, status %lx\n", (unsigned long)status);
          return FALSE;
        }
      break;
    case DLL_PROCESS_DETACH:
      WINE_UNIX_CALL (unix_detach, NULL);
      break;
    }

  return TRUE;
}
