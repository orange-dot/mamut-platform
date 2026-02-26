#include "mamut/mamut_comm.h"

mamut_err_t mamut_comm_init(void) { return MAMUT_OK; }

mamut_err_t mamut_comm_send(const uint8_t *data, uint32_t len) {
    (void)data; (void)len;
    return MAMUT_OK;
}

mamut_err_t mamut_comm_recv(uint8_t *data, uint32_t *len, uint32_t timeout_ms) {
    (void)data; (void)len; (void)timeout_ms;
    return MAMUT_OK;
}
