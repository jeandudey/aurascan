#ifndef __FREETRACK_WINE_BRIDGE_SHM_H__
#define __FREETRACK_WINE_BRIDGE_SHM_H__

#include <stddef.h>
#include <stdbool.h>

struct wine_shm;
struct shm_posix;

struct wine_shm *
wine_shm_create (const char *shm_name, const char *mutex_name, size_t len);

void
wine_shm_destroy (struct wine_shm *shm);

bool
wine_shm_lock (struct wine_shm *shm);

bool
wine_shm_unlock (struct wine_shm *shm);

void
wine_create_registry_key (bool use_freetrack, bool use_npclient,
                          const char *libdir);

#endif
