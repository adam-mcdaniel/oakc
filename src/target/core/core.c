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


///////////////////////////////////////////////////////////////////////
///////////////////////////// Error codes /////////////////////////////
///////////////////////////////////////////////////////////////////////
const int STACK_HEAP_COLLISION = 1;
const int NO_FREE_MEMORY       = 2;
const int STACK_UNDERFLOW      = 3;

// Fatal error handler. Always exits program.
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

///////////////////////////////////////////////////////////////////////
///////////////////////////// Debug Info //////////////////////////////
///////////////////////////////////////////////////////////////////////
// Print out the state of the virtual machine's stack and heap
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


/////////////////////////////////////////////////////////////////////////
///////////////////// Stack manipulation operations /////////////////////
/////////////////////////////////////////////////////////////////////////
// Push a number onto the stack
void machine_push(machine *vm, double n) {
    // If the memory at the stack pointer is allocated on the heap,
    // then the stack pointer has collided with the heap.
    // The program cannot continue without undefined behaviour,
    // so the program must panic.
    if (vm->allocated[vm->stack_ptr])
        panic(STACK_HEAP_COLLISION);
    
    // If the memory isn't allocated, simply push the value onto the stack.
    vm->memory[vm->stack_ptr++] = n;
}

// Pop a number from the stack
double machine_pop(machine *vm) {
    // If the stack pointer can't decrement any further,
    // the stack has underflowed.

    // It is not possible for pure Oak to generate code that will
    // cause a stack underflow. Foreign functions, or errors in
    // the virtual machine implementation are SOLELY responsible
    // for a stack underflow.
    if (vm->stack_ptr == 0) {
        panic(STACK_UNDERFLOW);
    }
    // Get the popped value
    double result = vm->memory[--vm->stack_ptr];
    // Overwrite the position on the stack with a zero
    vm->memory[vm->stack_ptr] = 0;
    return result;
}

////////////////////////////////////////////////////////////////////////
////////////////////// Constructor and destructor //////////////////////
////////////////////////////////////////////////////////////////////////
// Create new virtual machine
machine *machine_new(int global_scope_size, int capacity) {
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

    for (i=0; i<global_scope_size; i++)
        machine_push(result, 0);

    result->base_ptr = 0;

    return result;
}

// Free the virtual machine's memory. This is called at the end of the program.
void machine_drop(machine *vm) {
    // machine_dump(vm);
    free(vm->memory);
    free(vm->allocated);
}

////////////////////////////////////////////////////////////////////////
////////////////////// Function memory management //////////////////////
////////////////////////////////////////////////////////////////////////
// Push the base pointer onto the stack
void machine_load_base_ptr(machine *vm) {
    // Get the virtual machine's current base pointer value,
    // and push it onto the stack.
    machine_push(vm, vm->base_ptr);
}

// Establish a new stack frame for a function with `arg_size`
// number of cells as arguments.
void machine_establish_stack_frame(machine *vm, int arg_size, int local_scope_size) {
    // Allocate some space to store the arguments' cells for later
    double *args = malloc(arg_size * sizeof(double));
    int i;
    // Pop the arguments' values off of the stack
    for (i=arg_size-1; i>=0; i--)
        args[i] = machine_pop(vm);

    // Push the current base pointer onto the stack so that
    // when this function returns, it will be able to resume
    // the current stack frame
    machine_load_base_ptr(vm);

    // Set the base pointer to the current stack pointer to 
    // begin the stack frame at the current position on the stack.
    vm->base_ptr = vm->stack_ptr;

    // Allocate space for all the variables used in the local scope on the stack
    for (i=0; i<local_scope_size; i++)
        machine_push(vm, 0);

    // Push the arguments back onto the stack for use by the current function
    for (i=0; i<arg_size; i++)
        machine_push(vm, args[i]);

    // Free the space used to temporarily store the supplied arguments.
    free(args);
}

// End a stack frame for a function with `return_size` number of cells
// to return, and resume the parent stack frame.
void machine_end_stack_frame(machine *vm, int return_size, int local_scope_size) {
    // Allocate some space to store the returned cells for later
    double *return_val = malloc(return_size * sizeof(double));
    int i;
    // Pop the returned values off of the stack
    for (i=return_size-1; i>=0; i--)
        return_val[i] = machine_pop(vm);

    // Discard the memory setup by the stack frame
    for (i=0; i<local_scope_size; i++)
        machine_pop(vm);
    
    // Retrieve the parent function's base pointer to resume the function
    vm->base_ptr = machine_pop(vm);

    // Finally, push the returned value back onto the stack for use by
    // the parent function.
    for (i=0; i<return_size; i++)
        machine_push(vm, return_val[i]);

    // Free the space used to temporarily store the returned value.
    free(return_val);
}


/////////////////////////////////////////////////////////////////////////
///////////////////// Pointer and memory operations /////////////////////
/////////////////////////////////////////////////////////////////////////
// Pop the `size` parameter off of the stack, and return a pointer to `size` number of free cells.
int machine_allocate(machine *vm) {    
    // Get the size of the memory to allocate on the heap
    int i, size=machine_pop(vm), addr=0, consecutive_free_cells=0;

    // Starting at the end of the memory tape, find `size`
    // number of consecutive cells that have not yet been
    // allocated.
    for (i=vm->capacity-1; i>vm->stack_ptr; i--) {
        // If the memory hasn't been allocated, increment the counter.
        // Otherwise, reset the counter.
        if (!vm->allocated[i]) consecutive_free_cells++;
        else consecutive_free_cells = 0;

        // After we've found an address with the proper amount of memory left,
        // return the address.
        if (consecutive_free_cells == size) {
            addr = i;
            break;
        }
    }

    // If the address is less than the stack pointer,
    // the the heap must be full.
    // The program cannot continue without undefined behavior in this state.
    if (addr <= vm->stack_ptr)
        panic(NO_FREE_MEMORY);
    
    // Mark the address as allocated
    for (i=0; i<size; i++)
        vm->allocated[addr+i] = true;

    // Push the address onto the stack
    machine_push(vm, addr);
    return addr;
}

// Pop the `address` and `size` parameters off of the stack, and free the memory at `address` with size `size`.
void machine_free(machine *vm) {
    // Get the address and size to free from the stack
    int i, addr=machine_pop(vm), size=machine_pop(vm);

    // Mark the memory as unallocated, and zero each of the cells
    for (i=0; i<size; i++) {
        vm->allocated[addr+i] = false;
        vm->memory[addr+i] = 0;
    }
}

// Pop an `address` parameter off of the stack, and a `value` parameter with size `size`.
// Then store the `value` parameter at the memory address `address`.
void machine_store(machine *vm, int size) {
    // Pop an address off of the stack
    int i, addr=machine_pop(vm);

    // Pop `size` number of cells from the stack,
    // and store them at the address in the same order they were
    // pushed onto the stack.
    for (i=size-1; i>=0; i--) vm->memory[addr+i] = machine_pop(vm);
}

// Pop an `address` parameter off of the stack, and push the value at `address` with size `size` onto the stack.
void machine_load(machine *vm, int size) {
    int i, addr=machine_pop(vm);
    for (i=0; i<size; i++) machine_push(vm, vm->memory[addr+i]);
}

// Add the topmost numbers on the stack
void machine_add(machine *vm) {
    machine_push(vm, machine_pop(vm) + machine_pop(vm));
}

// Subtract the topmost number on the stack from the second topmost number on the stack
void machine_subtract(machine *vm) {
    double b = machine_pop(vm);
    double a = machine_pop(vm);
    machine_push(vm, a-b);
}

// Multiply the topmost numbers on the stack
void machine_multiply(machine *vm) {
    machine_push(vm, machine_pop(vm) * machine_pop(vm));
}

// Divide the second topmost number on the stack by the topmost number on the stack
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


