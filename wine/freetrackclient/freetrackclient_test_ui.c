/**
 * SPDX-FileCopyrightText: 2009 Tulthix, uglyDwarf
 * SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
 * SPDX-License-Identifier: MIT
 *
 * Extracted from Linuxtrack:
 * - <https://github.com/uglyDwarf/linuxtrack>
 */

#include <stdio.h>
#include <strsafe.h>
#include <windows.h>

#include "freetrackclient.h"
#include "resource.h"

static FreeTrackProcs Procs;
static HMODULE FreeTrackClient = NULL;
static UINT_PTR Timer;

static VOID CALLBACK
TimerProc (HWND Window, UINT Msg, UINT_PTR EventID, DWORD Time)
{
  CHAR Buf[64];
  FreeTrackData FTData;

  if (!Procs.FTGetData (&FTData))
    return;

  SetDlgItemInt (Window, IDC_YAW, FTData.Yaw, TRUE);
  SetDlgItemInt (Window, IDC_PITCH, FTData.Pitch, TRUE);
  SetDlgItemInt (Window, IDC_ROLL, FTData.Roll, TRUE);
  SetDlgItemInt (Window, IDC_X, FTData.X, TRUE);
  SetDlgItemInt (Window, IDC_Y, FTData.Y, TRUE);
  SetDlgItemInt (Window, IDC_Z, FTData.Z, TRUE);

  SetDlgItemInt (Window, IDC_RYAW, FTData.RawYaw, TRUE);
  SetDlgItemInt (Window, IDC_RPITCH, FTData.RawPitch, TRUE);
  SetDlgItemInt (Window, IDC_RROLL, FTData.RawRoll, TRUE);
  SetDlgItemInt (Window, IDC_RX, FTData.RawX, TRUE);
  SetDlgItemInt (Window, IDC_RY, FTData.RawY, TRUE);
  SetDlgItemInt (Window, IDC_RZ, FTData.RawZ, TRUE);

  StringCbPrintfA (Buf, sizeof (Buf), "(%.2f, %.2f)", FTData.X1, FTData.Y1);
  SetDlgItemText (Window, IDC_PT0, Buf);

  StringCbPrintfA (Buf, sizeof (Buf), "(%.2f, %.2f)", FTData.X2, FTData.Y2);
  SetDlgItemText (Window, IDC_PT1, Buf);

  StringCbPrintfA (Buf, sizeof (Buf), "(%.2f, %.2f)", FTData.X3, FTData.Y3);
  SetDlgItemText (Window, IDC_PT2, Buf);

  StringCbPrintfA (Buf, sizeof (Buf), "(%.2f, %.2f)", FTData.X4, FTData.Y4);
  SetDlgItemText (Window, IDC_PT3, Buf);

  StringCbPrintfA (Buf, sizeof (Buf), "%dx%d", FTData.CamWidth,
		   FTData.CamHeight);
  SetDlgItemText (Window, IDC_RES, Buf);

  SetDlgItemInt (Window, IDC_NUM, FTData.DataID, TRUE);
}

static BOOL
Start (HWND Window)
{
  CHAR Title[1024];

  if (FreeTrackClient)
    FreeLibrary (FreeTrackClient);

  if (!FTLoadDll (&Procs, &FreeTrackClient))
    {
      MessageBoxA (0, "Failed to load FreeTrackClient DLL", "Error", 0);
      return FALSE;
    }

  printf ("FreeTrackClient DLL Loaded.\n");
  printf ("DLL Provider: %s\n", Procs.FTProvider ());
  printf ("DLL Version: %s\n", Procs.FTGetDllVersion ());

  GetDlgItemText (Window, IDC_TITLE, Title, sizeof (Title) - 4);
  Procs.FTReportName (Title);

  if (Timer)
    KillTimer (Window, Timer);
  Timer = SetTimer (Window, 0, 50, TimerProc);

  return TRUE;
}

static BOOL
CommandProc (HWND Window, WORD Cmd)
{
  switch (Cmd)
    {
    case IDQUIT:
      FreeLibrary (FreeTrackClient);
      EndDialog (Window, 0);
      return TRUE;
    case IDC_START:
      Start (Window);
      return TRUE;
    }

  return FALSE;
}

static BOOL CALLBACK
DialogProc (HWND DialogWindow, UINT Msg, WPARAM WParam, LPARAM LParam)
{
  switch (Msg)
    {
    case WM_INITDIALOG:
      SetDlgItemText (DialogWindow, IDC_TITLE, "Default");
      return TRUE;

    case WM_CLOSE:
      EndDialog (DialogWindow, 0);
      return TRUE;

    case WM_COMMAND:
      return CommandProc (DialogWindow, LOWORD (WParam));
    }

  return FALSE;
}

int APIENTRY
WinMain (HINSTANCE Instance, HINSTANCE PrevInstance, LPSTR CmdLine,
	 int ShowCmd)
{
  (void) PrevInstance;
  (void) CmdLine;
  (void) ShowCmd;

  return DialogBox (Instance, MAKEINTRESOURCE (IDD_DIALOG1), NULL,
		    (DLGPROC) DialogProc);
}
