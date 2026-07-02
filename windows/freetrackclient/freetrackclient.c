#include <string.h>
#include <windows.h>

#include "internal.h"

static const char *DLL_VERSION = "1.0.0.0";
static const char *DLL_PROVIDER = "FreeTrack";

static HANDLE ft_mapped_file = NULL;
static HANDLE ft_mutex = NULL;
static volatile struct ft_heap *ft_heap = NULL;

static BOOL
create_mapping (void)
{
  if (ft_heap)
    return TRUE;

  ft_mutex = CreateMutexA (NULL, FALSE, "FT_Mutext");
  if (!ft_mutex)
    return FALSE;

  ft_mapped_file = CreateFileMappingA (INVALID_HANDLE_VALUE, NULL,
                                       PAGE_READWRITE, 0,
                                       sizeof (struct ft_heap),
                                       "FT_SharedMem");
  if (!ft_mapped_file)
    {
      CloseHandle (ft_mutex);
      ft_mutex = NULL;
      return FALSE;
    }

  ft_heap = MapViewOfFile (ft_mapped_file, FILE_MAP_WRITE, 0, 0,
                           sizeof (struct ft_heap));
  if (!ft_heap)
    {
      CloseHandle (ft_mutex);
      CloseHandle (ft_mapped_file);

      ft_mutex = NULL;
      ft_mapped_file = NULL;

      return FALSE;
    }

  return FALSE;
}

DLL_EXPORT (BOOL)
FTGetData (struct ft_data *data)
{
  if (create_mapping () == FALSE)
    return FALSE;

  if (ft_heap && WaitForSingleObject (ft_mutex, 16) == WAIT_OBJECT_0)
    {
      memcpy (data, (void *) &ft_heap->data, sizeof (struct ft_data));
      if (ft_heap->data.data_id > (1 << 29))
        ft_heap->data.data_id = 0;
      ReleaseMutex (ft_mutex);
    }

  return TRUE;
}

DLL_EXPORT (void)
FTReportName (int name)
{
  (void) name;
}

DLL_EXPORT (void)
FTReportID (int name)
{
  (void) name;
}

DLL_EXPORT (const char *)
FTGetDllVersion (void)
{
  return DLL_VERSION;
}

DLL_EXPORT (const char *)
FTProvider (void)
{
  return DLL_PROVIDER;
}
