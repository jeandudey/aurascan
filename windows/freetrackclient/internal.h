#ifndef __FREETRACK_CLIENT_H__
#define __FREETRACK_CLIENT_H__

#include <inttypes.h>

#if defined(_MSC_VER) && !defined(_WIN64)
#define DLL_EXPORT(t) t __stdcall
#else
#define DLL_EXPORT(t) __declspec(dllexport) t
#endif

struct ft_data {
  uint32_t data_id;
  int32_t cam_width;
  int32_t cam_height;
  float yaw;
  float pitch;
  float roll;
  float x;
  float y;
  float z;
  float raw_yaw;
  float raw_pitch;
  float raw_roll;
  float raw_x;
  float raw_y;
  float raw_z;
  float x1;
  float y1;
  float x2;
  float y2;
  float x3;
  float y3;
  float x4;
  float y4;
};

struct ft_heap {
  struct ft_data data;
  int32_t GameID;
  union
  {
    unsigned char table[8];
    int32_t table_ints[2];
  };
  int32_t GameID2;
};

#endif /* __FREETRACK_CLIENT_H__ */
