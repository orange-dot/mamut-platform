#include "mamut/mamut_diverter.h"

static mamut_diverter_state_t stub_state = MAMUT_DIVERTER_IDLE;
static mamut_direction_t stub_target = MAMUT_DIR_STRAIGHT;

static mamut_err_t stub_init(mamut_diverter_id_t id) {
    (void)id;
    stub_state = MAMUT_DIVERTER_IDLE;
    return MAMUT_OK;
}

static mamut_err_t stub_activate(mamut_diverter_id_t id, mamut_direction_t target) {
    (void)id;
    stub_state = MAMUT_DIVERTER_ACTIVE;
    stub_target = target;
    return MAMUT_OK;
}

static mamut_err_t stub_deactivate(mamut_diverter_id_t id) {
    (void)id;
    stub_state = MAMUT_DIVERTER_IDLE;
    stub_target = MAMUT_DIR_STRAIGHT;
    return MAMUT_OK;
}

static mamut_diverter_status_t stub_get_status(mamut_diverter_id_t id) {
    (void)id;
    mamut_diverter_status_t s = {0};
    s.state = stub_state;
    s.active_target = stub_target;
    return s;
}

static mamut_test_result_t stub_self_test(mamut_diverter_id_t id) {
    (void)id;
    mamut_test_result_t r = { .passed = true, .error_code = 0 };
    return r;
}

const mamut_diverter_ops_t mamut_diverter_stub_ops = {
    .name = "stub",
    .init = stub_init,
    .activate = stub_activate,
    .deactivate = stub_deactivate,
    .get_status = stub_get_status,
    .self_test = stub_self_test,
};
