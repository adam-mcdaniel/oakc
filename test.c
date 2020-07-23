#include "oak.h"

#define push(n)     machine_push(vm, n);
#define pop         machine_pop(vm);

#define load(size)  machine_load(vm, size);
#define store(size) machine_store(vm, size);

#define add  machine_add(vm);
#define sub  machine_subtract(vm);
#define mul  machine_multiply(vm);
#define div  machine_divide(vm);

#define alloc machine_allocate(vm);
#define free  machine_free(vm);

#define init(vars, capacity) int main() { machine *vm = machine_new(vars, capacity);
#define end machine_drop(*vm); }

init(4, 16)
    push(5)
    push(0) store(1)
    push(11)
    push(1) store(1)

    push(0) load(1) push(1) load(1) add 
    push(2) store(1)

    push(3) alloc push(3) store(1)

    // push(3) push(3) load(1) free
    push(5) push(6) push(7)
    push(3) load(1) store(3)
    push(3) load(1) load(3)
    push(3) push(3) load(1) free
end