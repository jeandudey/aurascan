#ifndef __POSIXBRIDGE_H__
#define __POSIXBRIDGE_H__

struct posixbridge_shm {
    int fd;
    int len;
    void *mem;
};

bool
posixbridge_shm_open (struct posixbridge_shm *shm, const char *shm_name,
                      int len);

void
posixbridge_shm_close (struct posixbridge_shm *shm);

bool
posixbridge_shm_lock (struct posixbridge_shm *shm);

bool
posixbridge_shm_unlock (struct posixbridge_shm *shm);

#endif
