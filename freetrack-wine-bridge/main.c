#include <stdio.h>
#include <windows.h>

#include "shm.h"

int
main (const int argc, const char **argv)
{
  struct wine_shm *wine_shm;

  printf ("Hello, World!\n");

  wine_shm = wine_shm_create ("FT_SharedMem", "FT_Mutext", 512);
  if (!wine_shm)
    {
      fprintf (stderr, "Failed to create Wine shared memory\n");
      return -1;
    }

  // TODO.
  wine_create_registry_key (false, false, "/");

  while (1)
    {
      bool locked = wine_shm_lock (wine_shm);

      if (locked)
        wine_shm_unlock (wine_shm);

      /* Sleep for 4 ms, 250 Hz update rate */
      Sleep (4);
    }

  wine_shm_destroy (wine_shm);

  return 0;
}
