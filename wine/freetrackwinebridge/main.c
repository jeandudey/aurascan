#include <stdio.h>
#include <windows.h>

#include "freetrackclient.h"
#include "freetrackwinebridge.h"

#define WINDOW_CLASS_NAME "FreetrackWineBridge"

static HANDLE FTMemMap;
static HANDLE FTMutex;
static volatile FreeTrackData *FTData;
static volatile HANDLE *FTHandle;
static volatile LPSTR FTProgramName;

LRESULT CALLBACK
WndProc (HWND Window, UINT Msg, WPARAM Wp, LPARAM Lp)
{
  if (Msg == RegisterWindowMessageA (FT_PROGRAMID))
    {
      printf ("Received ProgramName: %s\n", FTProgramName);
      if (bridge_lock () && WaitForSingleObject (FTMutex, INFINITE) == WAIT_OBJECT_0)
        {
          CopyMemory ((void *)bridge_ptr()->program_name, FTProgramName, 100);
          ReleaseMutex (FTMutex);
          bridge_unlock ();
        }
    }

  return DefWindowProc (Window, Msg, Wp, Lp);
}

int APIENTRY
WinMain (HINSTANCE Instance, HINSTANCE PrevInstance, LPSTR CmdLine,
         int ShowCmd)
{
  HWND Window;
  WNDCLASSEXA WindowClass = { sizeof (WNDCLASSEX) };
  UINT MapLen;
  MSG Msg;
  volatile struct winebridge *ptr;

  WindowClass.lpfnWndProc = WndProc;
  WindowClass.hInstance = Instance;
  WindowClass.lpszClassName = WINDOW_CLASS_NAME;

  if (!RegisterClassExA (&WindowClass))
    {
      fprintf (stderr, "Failed to register window class.\n");
      return 1;
    }

  Window = CreateWindowExA (0, WINDOW_CLASS_NAME, "", 0, 0, 0, 0, 0,
                            HWND_MESSAGE, NULL, Instance, NULL);

  FTMutex = CreateMutexA (NULL, FALSE, FREETRACK_MUTEX);
  if (!FTMutex)
    {
      fprintf (stderr, "Failed to create %s mutex.\n", FREETRACK_MUTEX);
      DestroyWindow (Window);
      UnregisterClassA (WINDOW_CLASS_NAME, Instance);
      return 1;
    }

  /* Until setup is complete. */
  if (WaitForSingleObject (FTMutex, INFINITE) != WAIT_OBJECT_0)
    {
      fprintf (stderr, "Failed to lock %s.\n", FREETRACK_MUTEX);
      DestroyWindow (Window);
      UnregisterClassA (WINDOW_CLASS_NAME, Instance);
      CloseHandle (FTMutex);
      return 1;
    }

  MapLen = sizeof (FreeTrackData) + sizeof (HANDLE) + 100;
  FTMemMap = CreateFileMappingA (INVALID_HANDLE_VALUE, NULL, PAGE_READWRITE,
                                 0, MapLen, FT_MM_DATA);
  if (!FTMemMap)
    {
      fprintf (stderr, "Failed to create file memory map.\n");
      DestroyWindow (Window);
      UnregisterClassA (WINDOW_CLASS_NAME, Instance);
      ReleaseMutex (FTMutex);
      CloseHandle (FTMutex);
      return 1;
    }

  FTData = MapViewOfFile (FTMemMap, FILE_MAP_WRITE, 0, 0, MapLen);
  if (!FTData)
    {
      fprintf (stderr, "Failed to map view of file.\n");
      DestroyWindow (Window);
      UnregisterClassA (WINDOW_CLASS_NAME, Instance);
      ReleaseMutex (FTMutex);
      CloseHandle (FTMutex);
      return 1;
    }

  FTHandle = (volatile HANDLE *) ((BYTE *) FTData + sizeof (FreeTrackData));
  *FTHandle = Window;

  FTProgramName = (volatile LPSTR) ((BYTE *) FTHandle + sizeof (FTHandle));

  ReleaseMutex (FTMutex);

  if (bridge_open () != 0)
    {
      fprintf (stderr, "Failed to open bridge.\n");
      DestroyWindow (Window);
      UnregisterClassA (WINDOW_CLASS_NAME, Instance);
      ReleaseMutex (FTMutex);
      CloseHandle (FTMutex);
    }

  while (1)
    {
      while (PeekMessage (&Msg, NULL, 0, 0, PM_REMOVE))
        {
          TranslateMessage (&Msg);
          DispatchMessage (&Msg);
        }

      if (!bridge_lock ())
        continue;

      ptr = bridge_ptr ();

      if (ptr->stop != 0)
        {
          bridge_unlock ();
          break;
        }

      if (WaitForSingleObject (FTMutex, INFINITE) != WAIT_OBJECT_0)
        continue;

      FTData->CamWidth = ptr->cam_width;
      FTData->CamHeight = ptr->cam_height;
      FTData->Yaw = ptr->yaw;
      FTData->Pitch = ptr->pitch;
      FTData->Roll = ptr->roll;
      FTData->X = ptr->x;
      FTData->Y = ptr->y;
      FTData->Z = ptr->z;
      FTData->RawYaw = ptr->raw_yaw;
      FTData->RawPitch = ptr->raw_pitch;
      FTData->RawRoll = ptr->raw_roll;
      FTData->RawX = ptr->raw_x;
      FTData->RawY = ptr->raw_y;
      FTData->RawZ = ptr->raw_z;
      FTData->X1 = ptr->x1;
      FTData->Y1 = ptr->y1;
      FTData->X2 = ptr->x2;
      FTData->Y2 = ptr->y2;
      FTData->X3 = ptr->x4;
      FTData->Y3 = ptr->y4;
      FTData->DataID = ptr->data_id;

      ReleaseMutex (FTMutex);
      bridge_unlock ();

      Sleep (20);
    }

  *FTHandle = NULL;

  UnmapViewOfFile ((LPCVOID)FTData);
  CloseHandle (FTMemMap);
  CloseHandle (FTMutex);
  DestroyWindow (Window);
  UnregisterClassA (WINDOW_CLASS_NAME, Instance);
  bridge_close ();

  return 0;
}
