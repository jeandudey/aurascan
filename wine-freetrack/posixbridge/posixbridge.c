#include <fcntl.h>
#include <limits.h>
#include <string.h>
#include <sys/file.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <stddef.h>
#include <unistd.h>
#include <malloc.h>
#include <stdbool.h>

#include "posixbridge.h"

bool
posixbridge_shm_open (struct posixbridge_shm *shm, const char *shm_name,
                      int len)
{
  char filename[PATH_MAX + 2];

  if (!shm)
    return false;

  shm->len = 0;
  shm->fd = -1;
  shm->mem = NULL;

  strcpy (filename, "/");
  strcat (filename, shm_name);

  shm->fd = shm_open (filename, O_RDWR | O_CREAT, 0600);
  shm->len = len;
  if (shm->fd == -1)
    return false;

  if (ftruncate (shm->fd, shm->len) == -1)
    {
      close (shm->fd);
      shm->fd = -1;
      shm->len = 0;
      shm->mem = NULL;
      return false;
    }

  shm->mem = mmap (NULL, shm->len, PROT_READ | PROT_WRITE, MAP_SHARED,
                   shm->fd, 0);
  if (shm->mem == MAP_FAILED)
    {
      close (shm->fd);
      shm->fd = -1;
      shm->len = 0;
      shm->mem = NULL;
      return false;
    }

  return true;
}

void
posixbridge_shm_close (struct posixbridge_shm *shm)
{
  if (!shm)
    return;

  munmap (shm->mem, shm->len);
  close (shm->fd);
}

bool
posixbridge_shm_lock (struct posixbridge_shm *shm)
{
  if (!shm)
    return false;

  if (shm->fd == -1)
    return false;

  return flock (shm->fd, LOCK_EX | LOCK_NB) == 0;
}

bool
posixbridge_shm_unlock (struct posixbridge_shm *shm)
{
  if (!shm)
    return false;

  if (shm->fd == -1)
    return false;

  return flock (shm->fd, LOCK_UN) == 0;
}
