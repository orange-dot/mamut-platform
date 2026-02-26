#ifndef MAMUT_COMMON_H
#define MAMUT_COMMON_H

#include <stdint.h>
#include <stdbool.h>

typedef uint32_t mamut_zone_id_t;
typedef uint32_t mamut_diverter_id_t;

typedef enum {
    MAMUT_OK = 0,
    MAMUT_ERR_INVALID_PARAM,
    MAMUT_ERR_TIMEOUT,
    MAMUT_ERR_FAULT,
    MAMUT_ERR_NOT_READY,
} mamut_err_t;

typedef enum {
    MAMUT_DIR_STRAIGHT = 0,
    MAMUT_DIR_LEFT,
    MAMUT_DIR_RIGHT,
} mamut_direction_t;

#endif /* MAMUT_COMMON_H */
