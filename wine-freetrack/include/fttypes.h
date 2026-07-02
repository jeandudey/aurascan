#ifndef __FTTYPES_H__
#define __FTTYPES_H__

#include <inttypes.h>

typedef struct FTData {
    uint32_t DataID;
    int32_t CamWidth;
    int32_t CamHeight;
    float  Yaw;
    float  Pitch;
    float  Roll;
    float  X;
    float  Y;
    float  Z;
    float  RawYaw;
    float  RawPitch;
    float  RawRoll;
    float  RawX;
    float  RawY;
    float  RawZ;
    float  X1;
    float  Y1;
    float  X2;
    float  Y2;
    float  X3;
    float  Y3;
    float  X4;
    float  Y4;
} FTData;

typedef struct FTHeap__ {
    FTData data;
    int32_t GameID;
    union
    {
        unsigned char table[8];
        int32_t table_ints[2];
    };
    int32_t GameID2;
} FTHeap;

#endif
