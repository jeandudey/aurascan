#ifndef __WINE_FREETRACK_FREETRACKCLIENT_UNIXLIB_H__
#define __WINE_FREETRACK_FREETRACKCLIENT_UNIXLIB_H__

#include <stdarg.h>
#include <winternl.h>

#include "wine/unixlib.h"

#include "fttypes.h"

struct FTGetData_params {
  FTData *data;
};

enum unix_funcs {
    unix_FTGetData,
    unix_detach,
    funcs_count,
};

#endif /* __WINE_FREETRACK_FREETRACKCLIENT_UNIXLIB_H__ */
