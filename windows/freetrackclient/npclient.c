#include <windows.h>

#include "internal.h"

BOOL
DllMain (HINSTANCE dll, DWORD reason, LPVOID reserved)
{
  (void) reserved;

  switch (reason)
    {
    case DLL_PROCESS_ATTACH:
      DisableThreadLibraryCalls (dll);
      break;

    case DLL_PROCESS_DETACH:
      break;
    }

  return TRUE;
}

DLL_EXPORT (int)
NPPriv_ClientNotify (void)
{
  return 0;
}

DLL_EXPORT (int)
NPPriv_GetLastError (void)
{
  return 0;
}

DLL_EXPORT (int)
NPPriv_SetData (void)
{
  return 0;
}

DLL_EXPORT (int)
NPPriv_SetLastError (void)
{
  return 0;
}

DLL_EXPORT (int)
NPPriv_SetParameter (void)
{
  return 0;
}

DLL_EXPORT (int)
NPPriv_SetVersion (void)
{
  return 0;
}

DLL_EXPORT (int)
NP_GetData (void *data)
{
  return 0;
}
