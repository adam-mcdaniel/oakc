#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>

typedef struct machine {
    double* memory;
    bool*   allocated;
    int     capacity;
    int     stack_ptr;
    int     base_ptr;
} machine;


// Fatal error handler. Always exits program.
void panic(int code);

////////////////////////////////////////////////////////////////////////
////////////////////// Constructor and destructor //////////////////////
////////////////////////////////////////////////////////////////////////
// Create new virtual machine
machine *machine_new(int vars, int capacity);
// Free the virtual machine's memory. This is called at the end of the program.
void machine_drop(machine *vm);

void machine_load_base_ptr(machine *vm);
void machine_load_stack_ptr(machine *vm);
void machine_store_base_ptr(machine *vm);


/////////////////////////////////////////////////////////////////////////
///////////////////// Stack manipulation operations /////////////////////
/////////////////////////////////////////////////////////////////////////
// Push a number onto the stack
void machine_push(machine *vm, double n);
// Pop a number from the stack
double machine_pop(machine *vm);
// Add the topmost numbers on the stack
void machine_add(machine *vm);
// Subtract the topmost number on the stack from the second topmost number on the stack
void machine_subtract(machine *vm);
// Multiply the topmost numbers on the stack
void machine_multiply(machine *vm);
// Divide the second topmost number on the stack by the topmost number on the stack
void machine_divide(machine *vm);


/////////////////////////////////////////////////////////////////////////
///////////////////// Pointer and memory operations /////////////////////
/////////////////////////////////////////////////////////////////////////
// Pop the `size` parameter off of the stack, and return a pointer to `size` number of free cells.
int machine_allocate(machine *vm);
// Pop the `address` and `size` parameters off of the stack, and free the memory at `address` with size `size`.
void machine_free(machine *vm);
// Pop an `address` parameter off of the stack, and a `value` parameter with size `size`.
// Then store the `value` parameter at the memory address `address`.
void machine_store(machine *vm, int size);
// Pop an `address` parameter off of the stack, and push the value at `address` with size `size` onto the stack.
void machine_load(machine *vm, int size);

void prn(machine *vm);
void prs(machine *vm);
void prc(machine *vm);
void prend(machine *vm);
void getch(machine *vm);


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

    result->base_ptr = result->stack_ptr+1;

    return result;
}

void machine_dump(machine *vm) {
    int i;
    printf("stack: [ ");
    for (i=0; i<vm->stack_ptr; i++)
        printf("%g ", vm->memory[i]);
    for (i=vm->stack_ptr; i<vm->capacity; i++)
        printf("  ");
    printf("]\nheap:  [ ");
    for (i=0; i<vm->stack_ptr; i++)
        printf("  ");
    for (i=vm->stack_ptr; i<vm->capacity; i++)
        printf("%g ", vm->memory[i]);
    printf("]\nalloc: [ ");
    for (i=0; i<vm->capacity; i++)
        printf("%d ", vm->allocated[i]);
    printf("]\n");
    int total = 0;
    for (i=0; i<vm->capacity; i++)
        total += vm->allocated[i];
    printf("STACK SIZE    %d\n", vm->stack_ptr);
    printf("TOTAL ALLOC'D %d\n", total);
}

void machine_drop(machine *vm) {
    // machine_dump(vm);
    free(vm->memory);
    free(vm->allocated);
}

void machine_load_base_ptr(machine *vm) {
    machine_push(vm, vm->base_ptr);
}

void machine_establish_stack_frame(machine *vm, int arg_size, int local_scope_size) {
    // printf("\nBEGIN 1 => BASE_PTR %d\n", vm->base_ptr);
    // machine_dump(vm);
    int base_ptr_save = vm->base_ptr, original_stack_ptr = vm->stack_ptr;
    vm->base_ptr = vm->stack_ptr+1;

    machine_push(vm, base_ptr_save);
    for (int i=0; i<local_scope_size; i++) {
        machine_push(vm, 0);
    }
    for (int i=0; i<arg_size; i++) {
        machine_push(vm, vm->memory[original_stack_ptr-arg_size+i]);
    }
    // printf("\nBEGIN 2 => BASE_PTR %d\n", vm->base_ptr);
    // machine_dump(vm);
}

void machine_end_stack_frame(machine *vm, int arg_size, int local_scope_size, int return_size) {
    // printf("\nEND 1 => BASE_PTR %d\n", vm->base_ptr);
    // machine_dump(vm);
    int i, return_val_ptr = vm->stack_ptr - return_size;
    vm->stack_ptr -= local_scope_size;
    vm->base_ptr = machine_pop(vm);
    vm->stack_ptr -= arg_size;

    machine_push(vm, return_val_ptr);
    machine_load(vm, return_size);

    for (i=0; i<local_scope_size + arg_size; i++)
        vm->memory[vm->stack_ptr + i] = 0;
    // printf("\nEND 2 => BASE_PTR %d\n", vm->base_ptr);
    // machine_dump(vm);
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

void machine_sign(machine *vm) {
    double x = machine_pop(vm);
    if (x >= 0) {
        machine_push(vm, 1);
    } else {
        machine_push(vm, -1);
    }
}