#include <string.h>
#include <windows.h>
#include <stdio.h>

#include "freetrackclient.h"

#define ASSERT_STR(s1, s2) \
  do { \
    if (!(s1)) \
      { \
        fprintf(stderr, "String is NULL\n"); \
        return 1; \
      } \
    if (strcmp((s1), (s2)) != 0) \
      { \
        fprintf(stderr, "Expected %s, got %s\n", (s2), (s1)); \
        return 1; \
      } \
  } while (0)

int
main (const int argc, const char **argv)
{
  HMODULE FreeTrackClient;
  FreeTrackProcs Procs;
  LPCSTR DllVersion, Provider;
  FreeTrackData Data;

  if (!FTLoadDll (&Procs, &FreeTrackClient))
    {
      fprintf (stderr, "Failed to load FreeTrackClient.dll\n");
      return 1;
    }

  DllVersion = Procs.FTGetDllVersion ();
  ASSERT_STR (DllVersion, "1.0.0.0");

  Provider = Procs.FTProvider ();
  ASSERT_STR (Provider, "Freetrack");

  Procs.FTReportName ("freetrackclient_test.exe");

  if (!Procs.FTGetData (&Data))
    {
      fprintf (stderr, "FTGetData returned FALSE\n");
      return 1;
    }

  FreeLibrary (FreeTrackClient);
  return 0;
}
