#ifndef MAMUT_DIVERTER_H
#define MAMUT_DIVERTER_H

#include "mamut_common.h"

typedef enum {
    MAMUT_DIVERTER_IDLE = 0,
    MAMUT_DIVERTER_ACTIVE,
    MAMUT_DIVERTER_FAULT,
    MAMUT_DIVERTER_TESTING,
} mamut_diverter_state_t;

typedef struct {
    mamut_diverter_state_t state;
    mamut_direction_t active_target;
    uint32_t fault_code;
} mamut_diverter_status_t;

typedef struct {
    bool passed;
    uint32_t error_code;
} mamut_test_result_t;

/* Pluggable diverter operations table */
typedef struct {
    const char *name;
    mamut_err_t (*init)(mamut_diverter_id_t id);
    mamut_err_t (*activate)(mamut_diverter_id_t id, mamut_direction_t target);
    mamut_err_t (*deactivate)(mamut_diverter_id_t id);
    mamut_diverter_status_t (*get_status)(mamut_diverter_id_t id);
    mamut_test_result_t (*self_test)(mamut_diverter_id_t id);
} mamut_diverter_ops_t;

/* Registration and access */
mamut_err_t mamut_diverter_register(mamut_diverter_id_t id, const mamut_diverter_ops_t *ops);
const mamut_diverter_ops_t *mamut_diverter_get_ops(mamut_diverter_id_t id);

#endif /* MAMUT_DIVERTER_H */
