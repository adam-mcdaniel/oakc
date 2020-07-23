#ifndef OAK_H
#define OAK_H
#include <stdbool.h>
#include <stdio.h>

typedef struct machine {
    double* memory;
    bool*   allocated;
    int     capacity;
    int     stack_ptr;
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

#endif