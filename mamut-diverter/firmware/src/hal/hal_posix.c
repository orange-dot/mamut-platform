#include "mamut/mamut_hal.h"
#include <time.h>
#include <unistd.h>
#include <stdio.h>

mamut_err_t mamut_hal_init(void) {
    return MAMUT_OK;
}

uint32_t mamut_hal_millis(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint32_t)(ts.tv_sec * 1000 + ts.tv_nsec / 1000000);
}

void mamut_hal_delay_ms(uint32_t ms) {
    usleep(ms * 1000);
}

mamut_err_t mamut_hal_gpio_set(uint32_t pin, bool state) {
    (void)pin; (void)state;
    return MAMUT_OK;
}

bool mamut_hal_gpio_get(uint32_t pin) {
    (void)pin;
    return false;
}

mamut_err_t mamut_hal_uart_send(uint8_t port, const uint8_t *data, uint32_t len) {
    (void)port; (void)data; (void)len;
    return MAMUT_OK;
}

mamut_err_t mamut_hal_uart_recv(uint8_t port, uint8_t *data, uint32_t *len, uint32_t timeout_ms) {
    (void)port; (void)data; (void)len; (void)timeout_ms;
    return MAMUT_OK;
}
