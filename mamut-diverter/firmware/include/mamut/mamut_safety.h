#ifndef MAMUT_SAFETY_H
#define MAMUT_SAFETY_H

#include "mamut_common.h"

mamut_err_t mamut_safety_init(void);
bool mamut_safety_check(void);
mamut_err_t mamut_safety_estop(void);

#endif /* MAMUT_SAFETY_H */
