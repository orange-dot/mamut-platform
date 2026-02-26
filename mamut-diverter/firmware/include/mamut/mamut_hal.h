#ifndef MAMUT_HAL_H
#define MAMUT_HAL_H

#include "mamut_common.h"

/* Hardware Abstraction Layer */

mamut_err_t mamut_hal_init(void);
uint32_t mamut_hal_millis(void);
void mamut_hal_delay_ms(uint32_t ms);

/* GPIO */
mamut_err_t mamut_hal_gpio_set(uint32_t pin, bool state);
bool mamut_hal_gpio_get(uint32_t pin);

/* Communication */
mamut_err_t mamut_hal_uart_send(uint8_t port, const uint8_t *data, uint32_t len);
mamut_err_t mamut_hal_uart_recv(uint8_t port, uint8_t *data, uint32_t *len, uint32_t timeout_ms);

#endif /* MAMUT_HAL_H */
