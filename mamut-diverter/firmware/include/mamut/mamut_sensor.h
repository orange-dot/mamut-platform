#ifndef MAMUT_SENSOR_H
#define MAMUT_SENSOR_H

#include "mamut_common.h"

typedef enum {
    MAMUT_SENSOR_PHOTOELECTRIC = 0,
    MAMUT_SENSOR_BARCODE,
    MAMUT_SENSOR_WEIGHT,
} mamut_sensor_type_t;

mamut_err_t mamut_sensor_init(void);
bool mamut_sensor_read(uint32_t sensor_id);

#endif /* MAMUT_SENSOR_H */
