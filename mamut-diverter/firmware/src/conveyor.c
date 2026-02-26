#include "mamut/mamut_conveyor.h"

mamut_err_t mamut_conveyor_init(void) {
    return MAMUT_OK;
}

mamut_err_t mamut_zone_set_speed(mamut_zone_id_t id, uint16_t speed_mm_per_s) {
    (void)id;
    (void)speed_mm_per_s;
    return MAMUT_OK;
}

mamut_zone_status_t mamut_zone_get_status(mamut_zone_id_t id) {
    mamut_zone_status_t status = {0};
    status.id = id;
    status.state = MAMUT_ZONE_IDLE;
    return status;
}
