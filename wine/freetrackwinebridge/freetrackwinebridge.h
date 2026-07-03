#ifndef __FREETRACKWINEBRIDGE_H__
#define __FREETRACKWINEBRIDGE_H__

#include <inttypes.h>

#define FREETRACKWINEBRIDGE_SHM "/freetrackwinebridge-shm"

struct winebridge {
  uint32_t data_id;
  int32_t cam_width;
  int32_t cam_height;
  float yaw, pitch, roll;
  float x, y, z;
  float raw_yaw, raw_pitch, raw_roll;
  float raw_x, raw_y, raw_z;
  float x1, y1, x2, y2, x3, y3, x4, y4;
  char program_name[100];
  uint32_t stop;
};

int
bridge_open (void);

int
bridge_lock (void);

int
bridge_unlock (void);

volatile struct winebridge *
bridge_ptr (void);

void
bridge_close (void);

#endif /* __FREETRACKWINEBRIDGE_H__ */
