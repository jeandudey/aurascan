#include <fcntl.h>
#include <sys/file.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <stddef.h>
#include <unistd.h>

#include "freetrackwinebridge.h"

static int fd = -1;
static volatile struct winebridge *ptr = NULL;

int
bridge_open (void)
{
  fd = shm_open(FREETRACKWINEBRIDGE_SHM, O_RDWR | O_CREAT, 0600);
  if (fd == -1)
    return -1;

  if (ftruncate (fd, sizeof (struct winebridge)) == -1)
    return -1;

  ptr = mmap (NULL, sizeof (struct winebridge), PROT_READ | PROT_WRITE,
              MAP_SHARED, fd, 0);
  if (ptr == NULL)
    return -1;

  return 0;
}

int
bridge_lock (void)
{
  return flock (fd, LOCK_EX) == 0;
}

int
bridge_unlock (void)
{
  return flock (fd, LOCK_UN) == 0;
}

volatile struct winebridge *
bridge_ptr (void)
{
  return ptr;
}

void
bridge_close (void)
{
  if (ptr)
    munmap ((void *)ptr, sizeof (struct winebridge));

  if (fd != -1)
    close (fd);
}
