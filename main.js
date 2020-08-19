///////////////////////////////////////////////////////////////////////
///////////////////////////// Error codes /////////////////////////////
///////////////////////////////////////////////////////////////////////
const STACK_HEAP_COLLISION = 1;
const NO_FREE_MEMORY = 2;
const STACK_UNDERFLOW = 3;
// Fatal error handler. Always exits program.
function panic(code) {
    let message = "panic: ";
    switch (code) {
        case 1:
            message += "stack and heap collision during push";
            break;
        case 2:
            message += "no free memory left";
            break;
        case 3:
            message += "stack underflow";
            break;
        default: message += "unknown error code";
    }
    message += "\n";
    //throwing an error is the closest thing JavaScript has to exit() afaik
    throw new Error(message);
}
// Create new virtual machine
function machine_new(vars, capacity) {
    let result = {
        capacity: capacity,
        memory: Array(capacity),
        allocated: Array(capacity),
        stack_ptr: 0,
        base_ptr: 0
    };
    //initialize the memory and allocated arrays
    for (let i = 0; i < capacity; i++) {
        result.memory[i] = 0;
        result.allocated[i] = false;
    }
    for (let i = 0; i < vars; i++)
        machine_push(result, 0);
    return result;
}
// Print out the state of the virtual machine's stack and heap
function machine_dump(vm) {
    let i;
    console.log("stack: [ ");
    for (i = 0; i < vm.stack_ptr; i++)
        console.log(vm.memory[i]);
    for (i = vm.stack_ptr; i < vm.capacity; i++)
        console.log("  ");
    console.log("]\nheap:  [ ");
    for (i = 0; i < vm.stack_ptr; i++)
        console.log("  ");
    for (i = vm.stack_ptr; i < vm.capacity; i++)
        console.log(`${vm.memory[i]} `);
    console.log("]\nalloc: [ ");
    for (i = 0; i < vm.capacity; i++)
        console.log(`${vm.allocated[i]} `);
    console.log("]\n");
    let total = 0;
    for (i = 0; i < vm.capacity; i++)
        total += vm.allocated[i] ? 1 : 0;
    console.log(`STACK SIZE	${vm.stack_ptr}\n`);
    console.log(`TOTAL ALLOC'D ${total}\n`);
}
// Free the virtual machine's memory. This is called at the end of the program.
function machine_drop(vm) {
    //JS doesn't have manual memory management, so this function does nothing
    //free(vm.memory);
    //free(vm.allocated);
}
function machine_load_base_ptr(vm) {
    // Get the virtual machine's current base pointer value,
    // and push it onto the stack.
    machine_push(vm, vm.base_ptr);
}
function machine_establish_stack_frame(vm, arg_size, local_scope_size) {
    // Allocate some space to store the arguments' cells for later
    let args = Array(arg_size);
    let i;
    // Pop the arguments' values off of the stack
    for (i = arg_size - 1; i >= 0; i--)
        args[i] = machine_pop(vm);
    // Push the current base pointer onto the stack so that
    // when this function returns, it will be able to resume
    // the current stack frame
    machine_load_base_ptr(vm);
    // Set the base pointer to the current stack pointer to 
    // begin the stack frame at the current position on the stack.
    vm.base_ptr = vm.stack_ptr;
    // Allocate space for all the variables used in the local scope on the stack
    for (i = 0; i < local_scope_size; i++)
        machine_push(vm, 0);
    // Push the arguments back onto the stack for use by the current function
    for (i = 0; i < arg_size; i++)
        machine_push(vm, args[i]);
}
function machine_end_stack_frame(vm, return_size, local_scope_size) {
    // Allocate some space to store the returned cells for later
    let return_val = Array(return_size);
    let i;
    // Pop the returned values off of the stack
    for (i = return_size - 1; i >= 0; i--)
        return_val[i] = machine_pop(vm);
    // Discard the memory setup by the stack frame
    for (i = 0; i < local_scope_size; i++)
        machine_pop(vm);
    // Retrieve the parent function's base pointer to resume the function
    vm.base_ptr = machine_pop(vm);
    // Finally, push the returned value back onto the stack for use by
    // the parent function.
    for (i = 0; i < return_size; i++)
        machine_push(vm, return_val[i]);
}
// Push a number onto the stack
function machine_push(vm, n) {
    if (vm.allocated[vm.stack_ptr])
        panic(STACK_HEAP_COLLISION);
    vm.memory[vm.stack_ptr++] = n;
}
// Pop a number from the stack
function machine_pop(vm) {
    if (vm.stack_ptr === 0) {
        panic(STACK_UNDERFLOW);
    }
    let result = vm.memory[vm.stack_ptr - 1];
    vm.memory[--vm.stack_ptr] = 0;
    //--vm.stack_ptr;
    return result;
}
// Pop the `size` parameter off of the stack, and return a pointer to `size` number of free cells.
function machine_allocate(vm) {
    let size = machine_pop(vm);
    let addr = 0;
    let consecutive_free_cells = 0;
    for (let i = vm.capacity - 1; i > vm.stack_ptr; i--) {
        if (!vm.allocated[i])
            consecutive_free_cells++;
        else
            consecutive_free_cells = 0;
        if (consecutive_free_cells === size) {
            addr = i;
            break;
        }
    }
    if (addr <= vm.stack_ptr)
        panic(NO_FREE_MEMORY);
    for (let i = 0; i < size; i++)
        vm.allocated[addr + i] = true;
    machine_push(vm, addr);
    return addr;
}
// Pop the `address` and `size` parameters off of the stack, and free the memory at `address` with size `size`.
function machine_free(vm) {
    let addr = machine_pop(vm);
    let size = machine_pop(vm);
    for (let i = 0; i < size; i++) {
        vm.allocated[addr + i] = false;
        vm.memory[addr + i] = 0;
    }
}
// Pop an `address` parameter off of the stack, and a `value` parameter with size `size`.
// Then store the `value` parameter at the memory address `address`.
function machine_store(vm, size) {
    let addr = machine_pop(vm);
    for (let i = size - 1; i >= 0; i--)
        vm.memory[addr + i] = machine_pop(vm);
}
// Pop an `address` parameter off of the stack, and push the value at `address` with size
//`size` onto the stack.
function machine_load(vm, size) {
    let addr = machine_pop(vm);
    for (let i = 0; i < size; i++)
        machine_push(vm, vm.memory[addr + i]);
}
// Add the topmost numbers on the stack
function machine_add(vm) {
    machine_push(vm, machine_pop(vm) + machine_pop(vm));
}
// Subtract the topmost number on the stack from the second topmost number on the stack
function machine_subtract(vm) {
    let b = machine_pop(vm);
    let a = machine_pop(vm);
    machine_push(vm, a - b);
}
// Multiply the topmost numbers on the stack
function machine_multiply(vm) {
    machine_push(vm, machine_pop(vm) * machine_pop(vm));
}
// Divide the second topmost number on the stack by the topmost number on the stack
function machine_divide(vm) {
    let b = machine_pop(vm);
    let a = machine_pop(vm);
    machine_push(vm, a / b);
}
function machine_sign(vm) {
    let x = machine_pop(vm);
    if (x >= 0) {
        machine_push(vm, 1);
    }
    else {
        machine_push(vm, -1);
    }
} //print a number
function prn(vm) {
    let n = machine_pop(vm);
    console.log(n);
}
//print a null-terminated string
function prs(vm) {
    let addr = machine_pop(vm);
    //console.log always inserts a newline, so build the string first and then print
    let out = "";
    for (let i = addr; vm.memory[i]; i++) {
        out += String.fromCharCode(vm.memory[i]);
    }
    console.log(out);
}
//print a char
function prc(vm) {
    let n = machine_pop(vm);
    console.log(String.fromCharCode(n));
}
//print a newline
function prend(vm) {
    //console.log always inserts a newline
    console.log("");
}
async function getch(vm) {
    //https://stackoverflow.com/questions/44746592/is-there-a-way-to-write-async-await-code-that-responds-to-onkeypress-events
    async function readKey() {
        return new Promise(resolve => {
            window.addEventListener('keypress', resolve, { once: true });
        });
    }
    let key = (await readKey()).key;
    let ch;
    if (key === "Enter") { //make sure pressing enter always gives \n
        ch = "\n".charCodeAt(0);
    }
    else if (key.length > 1) { //if the key is not a single character (arrow keys, etc.)
        //find a way to make this non-recursive
        getch(vm);
    }
    else {
        ch = key.charCodeAt(0);
    }
    machine_push(vm, ch);
}
async function fn0(vm) {
    machine_establish_stack_frame(vm, 1, 21);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    await fn2(vm);
    await fn10(vm);
    machine_push(vm, 2);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 2);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    machine_push(vm, 13);
    machine_subtract(vm);
    machine_push(vm, 3);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_push(vm, 4);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 3);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 0);
        machine_push(vm, 0);
        machine_push(vm, 3);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 4);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 3);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_push(vm, 4);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 3);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 4);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 4);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_push(vm, 2);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    machine_push(vm, 10);
    machine_subtract(vm);
    machine_push(vm, 5);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_push(vm, 6);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 5);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 0);
        machine_push(vm, 0);
        machine_push(vm, 5);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 6);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 5);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_push(vm, 6);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 5);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 6);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 6);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_add(vm);
    machine_push(vm, 7);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_push(vm, 8);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 7);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 7);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 8);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 7);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_push(vm, 8);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 0);
        machine_push(vm, 0);
        machine_push(vm, 7);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 8);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 8);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    while (machine_pop(vm)) {
        await fn10(vm);
        machine_push(vm, 2);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 2);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
        machine_push(vm, 13);
        machine_subtract(vm);
        machine_push(vm, 9);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 1);
        machine_push(vm, 10);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 9);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
        while (machine_pop(vm)) {
            machine_push(vm, 0);
            machine_push(vm, 0);
            machine_push(vm, 9);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_store(vm, 1);
            machine_push(vm, 0);
            machine_push(vm, 10);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_store(vm, 1);
            machine_push(vm, 9);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_load(vm, 1);
        }
        machine_push(vm, 10);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
        while (machine_pop(vm)) {
            machine_push(vm, 1);
            machine_push(vm, 0);
            machine_push(vm, 9);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_store(vm, 1);
            machine_push(vm, 0);
            machine_push(vm, 10);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_store(vm, 1);
            machine_push(vm, 10);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_load(vm, 1);
        }
        machine_push(vm, 2);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
        machine_push(vm, 10);
        machine_subtract(vm);
        machine_push(vm, 11);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 1);
        machine_push(vm, 12);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 11);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
        while (machine_pop(vm)) {
            machine_push(vm, 0);
            machine_push(vm, 0);
            machine_push(vm, 11);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_store(vm, 1);
            machine_push(vm, 0);
            machine_push(vm, 12);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_store(vm, 1);
            machine_push(vm, 11);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_load(vm, 1);
        }
        machine_push(vm, 12);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
        while (machine_pop(vm)) {
            machine_push(vm, 1);
            machine_push(vm, 0);
            machine_push(vm, 11);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_store(vm, 1);
            machine_push(vm, 0);
            machine_push(vm, 12);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_store(vm, 1);
            machine_push(vm, 12);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_load(vm, 1);
        }
        machine_add(vm);
        machine_push(vm, 13);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 1);
        machine_push(vm, 14);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 13);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
        while (machine_pop(vm)) {
            machine_push(vm, 1);
            machine_push(vm, 0);
            machine_push(vm, 13);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_store(vm, 1);
            machine_push(vm, 0);
            machine_push(vm, 14);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_store(vm, 1);
            machine_push(vm, 13);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_load(vm, 1);
        }
        machine_push(vm, 14);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
        while (machine_pop(vm)) {
            machine_push(vm, 0);
            machine_push(vm, 0);
            machine_push(vm, 13);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_store(vm, 1);
            machine_push(vm, 0);
            machine_push(vm, 14);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_store(vm, 1);
            machine_push(vm, 14);
            machine_load_base_ptr(vm);
            machine_add(vm);
            machine_load(vm, 1);
        }
    }
    machine_push(vm, 2);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    machine_push(vm, 121);
    machine_subtract(vm);
    machine_push(vm, 15);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_push(vm, 16);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 15);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 0);
        machine_push(vm, 0);
        machine_push(vm, 15);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 16);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 15);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_push(vm, 16);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 15);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 16);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 16);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_push(vm, 2);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    machine_push(vm, 89);
    machine_subtract(vm);
    machine_push(vm, 17);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_push(vm, 18);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 17);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 0);
        machine_push(vm, 0);
        machine_push(vm, 17);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 18);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 17);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_push(vm, 18);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 17);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 18);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 18);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_add(vm);
    machine_push(vm, 19);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_push(vm, 20);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 19);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 19);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 20);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 19);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_push(vm, 20);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 0);
        machine_push(vm, 0);
        machine_push(vm, 19);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 20);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 20);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_end_stack_frame(vm, 1, 21);
}
async function fn2(vm) {
    machine_establish_stack_frame(vm, 1, 2);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    await prs(vm);
    machine_end_stack_frame(vm, 0, 2);
}
async function fn3(vm) {
    machine_establish_stack_frame(vm, 1, 2);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    await fn2(vm);
    await prend(vm);
    machine_end_stack_frame(vm, 0, 2);
}
async function fn4(vm) {
    machine_establish_stack_frame(vm, 1, 2);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    await prn(vm);
    machine_end_stack_frame(vm, 0, 2);
}
async function fn5(vm) {
    machine_establish_stack_frame(vm, 1, 2);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    await fn4(vm);
    await prend(vm);
    machine_end_stack_frame(vm, 0, 2);
}
async function fn6(vm) {
    machine_establish_stack_frame(vm, 1, 2);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    await prc(vm);
    machine_end_stack_frame(vm, 0, 2);
}
async function fn7(vm) {
    machine_establish_stack_frame(vm, 1, 2);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    await fn6(vm);
    await prend(vm);
    machine_end_stack_frame(vm, 0, 2);
}
async function fn8(vm) {
    machine_establish_stack_frame(vm, 1, 4);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    machine_push(vm, 2);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_push(vm, 3);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 2);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 116);
        await fn6(vm);
        machine_push(vm, 114);
        await fn6(vm);
        machine_push(vm, 117);
        await fn6(vm);
        machine_push(vm, 101);
        await fn6(vm);
        machine_push(vm, 0);
        machine_push(vm, 2);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 3);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 2);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_push(vm, 3);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 102);
        await fn6(vm);
        machine_push(vm, 97);
        await fn6(vm);
        machine_push(vm, 108);
        await fn6(vm);
        machine_push(vm, 115);
        await fn6(vm);
        machine_push(vm, 101);
        await fn6(vm);
        machine_push(vm, 0);
        machine_push(vm, 2);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 3);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 3);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_end_stack_frame(vm, 0, 4);
}
async function fn9(vm) {
    machine_establish_stack_frame(vm, 1, 2);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    await fn8(vm);
    await prend(vm);
    machine_end_stack_frame(vm, 0, 2);
}
async function fn10(vm) {
    machine_establish_stack_frame(vm, 0, 1);
    await getch(vm);
    machine_end_stack_frame(vm, 1, 1);
}
async function fn1(vm) {
    machine_establish_stack_frame(vm, 0, 3);
    machine_push(vm, 3);
    await fn5(vm);
    machine_push(vm, 68);
    machine_push(vm, 111);
    machine_push(vm, 32);
    machine_push(vm, 121);
    machine_push(vm, 111);
    machine_push(vm, 117);
    machine_push(vm, 32);
    machine_push(vm, 108);
    machine_push(vm, 105);
    machine_push(vm, 107);
    machine_push(vm, 101);
    machine_push(vm, 32);
    machine_push(vm, 97);
    machine_push(vm, 112);
    machine_push(vm, 112);
    machine_push(vm, 108);
    machine_push(vm, 101);
    machine_push(vm, 115);
    machine_push(vm, 32);
    machine_push(vm, 40);
    machine_push(vm, 121);
    machine_push(vm, 47);
    machine_push(vm, 110);
    machine_push(vm, 41);
    machine_push(vm, 63);
    machine_push(vm, 32);
    machine_push(vm, 0);
    machine_push(vm, 0);
    machine_store(vm, 27);
    machine_push(vm, 0);
    await fn0(vm);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_push(vm, 2);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 89);
        machine_push(vm, 111);
        machine_push(vm, 117);
        machine_push(vm, 32);
        machine_push(vm, 108);
        machine_push(vm, 105);
        machine_push(vm, 107);
        machine_push(vm, 101);
        machine_push(vm, 32);
        machine_push(vm, 97);
        machine_push(vm, 112);
        machine_push(vm, 112);
        machine_push(vm, 108);
        machine_push(vm, 101);
        machine_push(vm, 115);
        machine_push(vm, 33);
        machine_push(vm, 0);
        machine_push(vm, 27);
        machine_store(vm, 17);
        machine_push(vm, 27);
        machine_push(vm, 0);
        machine_push(vm, 1);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 2);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 1);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    machine_push(vm, 2);
    machine_load_base_ptr(vm);
    machine_add(vm);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 89);
        machine_push(vm, 111);
        machine_push(vm, 117);
        machine_push(vm, 32);
        machine_push(vm, 100);
        machine_push(vm, 111);
        machine_push(vm, 110);
        machine_push(vm, 39);
        machine_push(vm, 116);
        machine_push(vm, 32);
        machine_push(vm, 108);
        machine_push(vm, 105);
        machine_push(vm, 107);
        machine_push(vm, 101);
        machine_push(vm, 32);
        machine_push(vm, 97);
        machine_push(vm, 112);
        machine_push(vm, 112);
        machine_push(vm, 108);
        machine_push(vm, 101);
        machine_push(vm, 115);
        machine_push(vm, 33);
        machine_push(vm, 0);
        machine_push(vm, 44);
        machine_store(vm, 23);
        machine_push(vm, 44);
        machine_push(vm, 0);
        machine_push(vm, 1);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 2);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_store(vm, 1);
        machine_push(vm, 2);
        machine_load_base_ptr(vm);
        machine_add(vm);
        machine_load(vm, 1);
    }
    await fn3(vm);
    machine_end_stack_frame(vm, 0, 3);
}
async function OAKmain() {
    let vm = machine_new(67, 579);
    await fn1(vm);
    machine_drop(vm);
}
OAKmain();
