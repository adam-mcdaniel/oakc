#include "oak.h"
#include <stdio.h>
#include <stdlib.h>

///////////////////////////////////////////////////////////////////////
///////////////////////////// Error codes /////////////////////////////
///////////////////////////////////////////////////////////////////////
const int STACK_HEAP_COLLISION = 1;
const int NO_FREE_MEMORY       = 2;
const int STACK_UNDERFLOW      = 3;

void panic(int code) {
    printf("panic: ");
    switch (code) {
        case 1: printf("stack and heap collision during push"); break;
        case 2: printf("no free memory left"); break;
        case 3: printf("stack underflow"); break;
        default: printf("unknown error code");
    }
    printf("\n");
    exit(code);
}

machine *machine_new(int vars, int capacity) {
    machine *result = malloc(sizeof(machine));
    result->capacity  = capacity;
    result->memory    = malloc(sizeof(double) * capacity);
    result->allocated = malloc(sizeof(bool)   * capacity);
    result->stack_ptr = 0;
    int i;
    for (i=0; i<capacity; i++) {
        result->memory[i] = 0;
        result->allocated[i] = false;
    }

    for (i=0; i<vars; i++)
        machine_push(result, 0);

    return result;
}

void machine_drop(machine *vm) {
    int i;
    printf("stack: [ ");
    for (i=0; i<vm->stack_ptr; i++)
        printf("%g ", vm->memory[i]);
    for (i=vm->stack_ptr; i<vm->capacity; i++)
        printf("  ");
    printf("]\n");

    printf("heap:  [ ");
    for (i=0; i<vm->stack_ptr; i++)
        printf("  ");
    for (i=vm->stack_ptr; i<vm->capacity; i++)
        printf("%g ", vm->memory[i]);
    printf("]\n");

    printf("alloc: [ ");
    for (i=0; i<vm->capacity; i++)
        printf("%d ", vm->allocated[i]);
    printf("]\n");

    int total = 0;
    for (i=0; i<vm->capacity; i++)
        total += vm->allocated[i];
    printf("STACK SIZE    %d\n", vm->stack_ptr);
    printf("TOTAL ALLOC'D %d\n", total);

    free(vm->memory);
    free(vm->allocated);
}

void machine_push(machine *vm, double n) {
    if (vm->allocated[vm->stack_ptr])
        panic(STACK_HEAP_COLLISION);
    vm->memory[vm->stack_ptr++] = n;
}

double machine_pop(machine *vm) {
    if (vm->stack_ptr == 0) {
        panic(STACK_UNDERFLOW);
    }
    double result = vm->memory[vm->stack_ptr-1];
    vm->memory[--vm->stack_ptr] = 0;
    return result;
}

int machine_allocate(machine *vm) {    
    int i, size=machine_pop(vm), addr=0, consecutive_free_cells=0;
    for (i=vm->capacity-1; i>vm->stack_ptr; i--) {
        if (!vm->allocated[i]) consecutive_free_cells++;
        else consecutive_free_cells = 0;

        if (consecutive_free_cells == size) {
            addr = i;
            break;
        }
    }

    if (addr <= vm->stack_ptr)
        panic(NO_FREE_MEMORY);
    
    for (i=0; i<size; i++)
        vm->allocated[addr+i] = true;

    machine_push(vm, addr);
    return addr;
}

void machine_free(machine *vm) {
    int i, addr=machine_pop(vm), size=machine_pop(vm);

    for (i=0; i<size; i++) {
        vm->allocated[addr+i] = false;
        vm->memory[addr+i] = 0;
    }
}

void machine_store(machine *vm, int size) {
    int i, addr=machine_pop(vm);

    for (i=size-1; i>=0; i--) vm->memory[addr+i] = machine_pop(vm);
}

void machine_load(machine *vm, int size) {
    int i, addr=machine_pop(vm);

    for (i=0; i<size; i++) machine_push(vm, vm->memory[addr+i]);
}

// void machine_reserve(machine *vm, int size, int argument_size) {
//     int i;
//     double *arguments_value = malloc(sizeof(double) * argument_size);
//     for (i=argument_size-1; i>=0; i--) arguments_value[i] = machine_pop(vm);
//     for (i=0; i<size; i++) machine_push(vm, 0);
//     for (i=0; i<argument_size; i++) machine_push(vm, arguments_value[i]);
//     free(arguments_value);
// }

// void machine_unreserve(machine *vm, int size, int return_size) {
//     int i;
//     double *return_value = malloc(sizeof(double) * return_size);
//     for (i=return_size-1; i>=0; i--) return_value[i] = machine_pop(vm);
//     for (i=0; i<size; i++) machine_pop(vm);
//     for (i=0; i<return_size; i++) machine_push(vm, return_value[i]);
//     free(return_value);
// }

void machine_add(machine *vm) {
    machine_push(vm, machine_pop(vm) + machine_pop(vm));
}

void machine_subtract(machine *vm) {
    double b = machine_pop(vm);
    double a = machine_pop(vm);
    machine_push(vm, a-b);
}

void machine_multiply(machine *vm) {
    machine_push(vm, machine_pop(vm) * machine_pop(vm));
}

void machine_divide(machine *vm) {
    double b = machine_pop(vm);
    double a = machine_pop(vm);
    machine_push(vm, a/b);
}

void prn(machine *vm) {
    double n = machine_pop(vm);
    printf("%g", n);
}

void prs(machine *vm) {
    double addr = machine_pop(vm);
    int i;
    for (i=addr; vm->memory[i]; i++) {
        printf("%c", (char)vm->memory[i]);
    }
}

void prc(machine *vm) {
    double n = machine_pop(vm);
    printf("%c", (char)n);
}

void prend(machine *vm) {
    printf("\n");
}

void getch(machine *vm) {
    machine_push(vm, getchar());
}
