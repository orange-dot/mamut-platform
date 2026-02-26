#ifndef MAMUT_COMM_H
#define MAMUT_COMM_H

#include "mamut_common.h"

mamut_err_t mamut_comm_init(void);
mamut_err_t mamut_comm_send(const uint8_t *data, uint32_t len);
mamut_err_t mamut_comm_recv(uint8_t *data, uint32_t *len, uint32_t timeout_ms);

#endif /* MAMUT_COMM_H */
