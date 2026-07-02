#include <stdio.h>
#if 0
#pragma makedep unix
#endif

#define WINE_UNIX_LIB

#include "unixlib.h"
#include "posixbridge.h"

#include <ntstatus.h>
#include <wine/debug.h>

WINE_DEFAULT_DEBUG_CHANNEL(freetrack);

static struct posixbridge_shm shm = {
  .fd = -1,
  .len = 0,
  .mem = NULL,
};
static BOOL map_failed = FALSE;

static BOOL
map_shared_memory (void)
{
  if (map_failed == TRUE)
    return FALSE;

  if (shm.mem)
    return TRUE;

  if (!posixbridge_shm_open (&shm, "freetrack-shm", sizeof (FTHeap)))
    {
      printf("ERROR");
      WINE_ERR ("failed to map aurascan-freetrack-shm memory\n");
      map_failed = TRUE;
      return FALSE;
    }

  return TRUE;
}

static NTSTATUS
wine_FTGetData (void *args)
{
  FTHeap *ftheap;
  struct FTGetData_params *params;

  if (map_shared_memory () == FALSE)
    return STATUS_UNSUCCESSFUL;

  if (posixbridge_shm_lock (&shm))
    {
      params = args;
      ftheap = shm.mem;
      memcpy (params->data, &ftheap->data, sizeof (FTData));

      if (ftheap->data.DataID > (1 << 29))
        ftheap->data.DataID = 0;

      if (!posixbridge_shm_unlock (&shm))
        WINE_WARN ("failed to unlock shm\n");

      return STATUS_SUCCESS;
    }

  return STATUS_CANT_WAIT;
}

static NTSTATUS
wine_detach (void *args)
{
  posixbridge_shm_close (&shm);
  shm.fd = -1;
  shm.len = 0;
  shm.mem = NULL;
  map_failed = FALSE;
  return TRUE;
}

const unixlib_entry_t __wine_unix_call_funcs[] = {
  wine_FTGetData,
  wine_detach,
};

C_ASSERT (ARRAYSIZE (__wine_unix_call_funcs) == funcs_count);
