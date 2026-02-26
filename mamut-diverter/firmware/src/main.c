#include "mamut/mamut_hal.h"
#include "mamut/mamut_conveyor.h"
#include "mamut/mamut_diverter.h"
#include "mamut/mamut_safety.h"
#include "mamut/mamut_comm.h"

int main(void) {
    mamut_hal_init();
    mamut_conveyor_init();
    mamut_safety_init();
    mamut_comm_init();

    while (1) {
        /* Main loop - stub */
        mamut_hal_delay_ms(10);
    }

    return 0;
}
