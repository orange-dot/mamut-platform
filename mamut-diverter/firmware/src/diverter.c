#include "mamut/mamut_diverter.h"
#include <string.h>

#define MAX_DIVERTERS 16

static struct {
    bool registered;
    const mamut_diverter_ops_t *ops;
} diverters[MAX_DIVERTERS];

mamut_err_t mamut_diverter_register(mamut_diverter_id_t id, const mamut_diverter_ops_t *ops) {
    if (id >= MAX_DIVERTERS || !ops) return MAMUT_ERR_INVALID_PARAM;
    diverters[id].ops = ops;
    diverters[id].registered = true;
    return MAMUT_OK;
}

const mamut_diverter_ops_t *mamut_diverter_get_ops(mamut_diverter_id_t id) {
    if (id >= MAX_DIVERTERS || !diverters[id].registered) return NULL;
    return diverters[id].ops;
}
