#include "mamut/mamut_diverter.h"
#include <assert.h>
#include <stdio.h>

/* Minimal test without Unity framework */

extern const mamut_diverter_ops_t mamut_diverter_stub_ops;

static void test_register_and_activate(void) {
    mamut_err_t err = mamut_diverter_register(0, &mamut_diverter_stub_ops);
    assert(err == MAMUT_OK);

    const mamut_diverter_ops_t *ops = mamut_diverter_get_ops(0);
    assert(ops != NULL);

    err = ops->init(0);
    assert(err == MAMUT_OK);

    err = ops->activate(0, MAMUT_DIR_LEFT);
    assert(err == MAMUT_OK);

    mamut_diverter_status_t status = ops->get_status(0);
    assert(status.state == MAMUT_DIVERTER_ACTIVE);
    assert(status.active_target == MAMUT_DIR_LEFT);

    err = ops->deactivate(0);
    assert(err == MAMUT_OK);

    status = ops->get_status(0);
    assert(status.state == MAMUT_DIVERTER_IDLE);

    printf("test_register_and_activate: PASSED\n");
}

static void test_self_test(void) {
    const mamut_diverter_ops_t *ops = mamut_diverter_get_ops(0);
    mamut_test_result_t result = ops->self_test(0);
    assert(result.passed == true);
    printf("test_self_test: PASSED\n");
}

int main(void) {
    test_register_and_activate();
    test_self_test();
    printf("All tests passed.\n");
    return 0;
}
