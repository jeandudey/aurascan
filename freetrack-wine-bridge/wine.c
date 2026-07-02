#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>

#include "shm.h"

struct wine_shm {
  HANDLE mutex;
  HANDLE mapped_file;
  LPVOID mem;
};

struct wine_shm *
wine_shm_create (const char *shm_name, const char *mutex_name, size_t len)
{
  DWORD len_high, len_low;
  struct wine_shm *shm;

  if (!shm_name)
    {
      fprintf (stderr, "shm_name is required\n");
      return NULL;
    }

  shm = malloc (sizeof (struct wine_shm));
  if (!shm)
    return NULL;

  if (mutex_name)
    {
      shm->mutex = CreateMutexA (NULL, FALSE, mutex_name);
      if (!shm->mutex)
        {
          fprintf (stderr, "Failed to create mutex with name %s\n", mutex_name);
          return NULL;
        }
    }

#ifdef _WIN64
  len_high = (DWORD)(len >> 32);
#else
  len_high = 0;
#endif
  len_low = (DWORD)len;

  shm->mapped_file = CreateFileMappingA (INVALID_HANDLE_VALUE, NULL,
                                         PAGE_READWRITE, len_high, len_low,
                                         shm_name);
  if (!shm->mapped_file)
    {
      fprintf (stderr, "Failed to create file mapping with name %s\n", shm_name);
      return NULL;
    }

  shm->mem = MapViewOfFile (shm->mapped_file, FILE_MAP_WRITE, 0, 0, len);
  if (!shm->mem)
    {
      fprintf (stderr, "Failed to create mapped view of file\n");
      return NULL;
    }

  return shm;
}

void
wine_shm_destroy (struct wine_shm *shm)
{
  if (!shm)
    return;

  if (shm->mem)
    {
      if (!UnmapViewOfFile (shm->mem)) {
        fprintf (stderr, "Failed to unmap view of file\n");
        return;
      }
    }

  if (shm->mapped_file)
    {
      if (!CloseHandle (shm->mapped_file)) {
        fprintf (stderr, "Failed to close mapped file\n");
        return;
      }
    }

  if (shm->mutex)
    {
      if (!CloseHandle (shm->mutex)) {
        fprintf (stderr, "Failed to close mutex\n");
        return;
      }
    }

  free (shm);
}

bool
wine_shm_lock (struct wine_shm *shm)
{
  if (!shm)
    return false;

  if (shm->mutex)
    return WaitForSingleObject (shm->mutex, INFINITE) == WAIT_OBJECT_0;

  return false;
}

bool
wine_shm_unlock (struct wine_shm *shm)
{
  if (!shm)
    return false;

  if (shm->mutex)
    return ReleaseMutex (shm->mutex);

  return false;
}

void *
wine_shm_mem (struct wine_shm *shm)
{
  if (!shm)
    return NULL;

  return shm->mem;
}

static void
write_path (const char *key, const char *subkey, bool path,
            const char *libdir)
{
  char dir[8192];
  HKEY hkpath;

  if (GetCurrentDirectoryA (8192, dir) < 8190)
    {
      if (RegCreateKeyExA (HKEY_CURRENT_USER, key, 0, NULL, 0, KEY_ALL_ACCESS,
                           NULL, &hkpath, NULL) == ERROR_SUCCESS)
        {
          for (int i = 0; dir[i]; i++)
            {
              if (dir[i] == '\\')
                dir[i] = '/';
            }

          strcat (dir, libdir);

          if (!path)
            dir[0] = '\0';

          RegSetValueExA (hkpath, subkey, 0, REG_SZ, (BYTE *)dir, strlen (dir) + 1);
          RegCloseKey (hkpath);
        }
    }
}

void
wine_create_registry_key (bool use_freetrack, bool use_npclient,
                          const char *libdir)
{
  write_path ("Software\\NaturalPoint\\NATURALPOINT\\NPClient Location",
              "Path", use_npclient, libdir);
  write_path ("Software\\Freetrack\\FreeTrackClient",
              "Path", use_freetrack, libdir);
}
