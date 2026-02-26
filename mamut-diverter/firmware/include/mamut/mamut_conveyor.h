#ifndef MAMUT_CONVEYOR_H
#define MAMUT_CONVEYOR_H

#include "mamut_common.h"

typedef enum {
    MAMUT_ZONE_IDLE = 0,
    MAMUT_ZONE_RUNNING,
    MAMUT_ZONE_FAULT,
    MAMUT_ZONE_STOPPED,
} mamut_zone_state_t;

typedef struct {
    mamut_zone_id_t id;
    mamut_zone_state_t state;
    uint16_t speed_mm_per_s;
    bool item_present;
} mamut_zone_status_t;

mamut_err_t mamut_conveyor_init(void);
mamut_err_t mamut_zone_set_speed(mamut_zone_id_t id, uint16_t speed_mm_per_s);
mamut_zone_status_t mamut_zone_get_status(mamut_zone_id_t id);

#endif /* MAMUT_CONVEYOR_H */
