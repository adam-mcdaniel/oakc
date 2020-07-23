#include "oak.h"
#include <stdio.h>

void square(machine *vm) {
    machine_push(vm, 0);
    machine_store(vm, 1);

    machine_push(vm, 0);
    machine_load(vm, 1);
    
    machine_push(vm, 0);
    machine_load(vm, 1);
    
    machine_multiply(vm);
}

int main() {
    machine *vm = machine_new(4, 16);
    machine_push(vm, 5);
    square(vm);
    printf("%g\n", machine_pop(vm));
    machine_drop(vm);
}