class Machine
	attr_accessor :memory, :allocated, :capacity, :stack_ptr, :base_ptr

	def initialize(capacity)
		@memory = Array.new(capacity, 0.0)
		@allocated = Array.new(capacity, false)
		@capacity = capacity
		@stack_ptr = 0
		@base_ptr = 0
	end
end


#########################################
############## Error codes ##############
#########################################
STACK_HEAP_COLLISION = 1
NO_FREE_MEMORY       = 2
STACK_UNDERFLOW      = 3

# Fatal error handler. Always exits program.
def panic(code)
    print("panic: ")
    case code
	when 1
		puts("stack and heap collision during push")
	when 2
		puts("no free memory left")
	when 3
		puts("stack underflow")
	else
		puts("unknown error code")
	end
    exit(code)
end

#########################################
############## Debug Info ###############
#########################################
# Print out the state of the virtual machine's stack and heap
def machine_dump(vm) {
	print("stack: [ ")
	vm.stack_ptr.times do |i|
		print(vm.memory[i])
	end
	for i in vm.stack_ptr..(vm.capacity-1)
		print("  ")
	end
    print("]\nheap:  [ ")
    vm.stack_ptr.times do |i|
		print("  ")
	end
	for i in vm.stack_ptr..(vm.capacity-1)
		print(vm.memory[i])
	end
    print("]\nalloc: [ ")
    vm.capacity.times do |i|
		print(vm.allocated[i])
	end
    puts("]")
    total = 0
    vm.capacity.times do |i|
		vm.allocated[i] ? total += 1
	end
    puts("STACK SIZE    #{vm.stack_ptr}")
    puts("TOTAL ALLOC'D #{total}")
}


###################################################
########## Stack manipulation operations ##########
###################################################
# Push a number onto the stack
def machine_push(vm, n)
    # If the memory at the stack pointer is allocated on the heap,
    # then the stack pointer has collided with the heap.
    # The program cannot continue without undefined behaviour,
    # so the program must panic.
    if vm.allocated[vm.stack_ptr]
        panic(STACK_HEAP_COLLISION)
	end
	# If the memory isn't allocated, simply push the value onto the stack.
	vm.memory[vm.stack_ptr] = n
	vm.stack_ptr += 1
end

# Pop a number from the stack
def machine_pop(vm)
    # If the stack pointer can't decrement any further,
    # the stack has underflowed.

    # It is not possible for pure Oak to generate code that will
    # cause a stack underflow. Foreign functions, or errors in
    # the virtual machine implementation are SOLELY responsible
    # for a stack underflow.
    if (vm.stack_ptr == 0) {
        panic(STACK_UNDERFLOW)
    end
	# Get the popped value
	vm.stack_ptr -= 1
    result = vm.memory[vm.stack_ptr]
    # Overwrite the position on the stack with a zero
    vm.memory[vm.stack_ptr] = 0
    return result
end

####################################
########### Constructor and destructor ###########
####################################
# Create new virtual machine
def machine_new(global_scope_size, capacity) {
    result = Machine.new(capacity)

    global_scope_size.times do
        machine_push(result, 0)
	end

    return result
end

# Free the virtual machine's memory. This is called at the end of the program.
def machine_drop(vm) {
    # machine_dump(vm)
    # free(vm.memory)
    # free(vm.allocated)
end

##################################################
########### Function memory management ###########
##################################################
# Push the base pointer onto the stack
def machine_load_base_ptr(vm) {
    # Get the virtual machine's current base pointer value,
    # and push it onto the stack.
    machine_push(vm, vm.base_ptr)
end

# Establish a new stack frame for a function with `arg_size`
# number of cells as arguments.
def machine_establish_stack_frame(vm, arg_size, local_scope_size) {
    # Allocate some space to store the arguments' cells for later
    args = new Array(arg_size, 0.0)
    # Pop the arguments' values off of the stack
    for i in (arg_size-1).downto(0)
        args[i] = machine_pop(vm)
	end
    # Push the current base pointer onto the stack so that
    # when this function returns, it will be able to resume
    # the current stack frame
    machine_load_base_ptr(vm)

    # Set the base pointer to the current stack pointer to 
    # begin the stack frame at the current position on the stack.
    vm.base_ptr = vm.stack_ptr

    # Allocate space for all the variables used in the local scope on the stack
    local_scope_size.times do
        machine_push(vm, 0)
	end
    # Push the arguments back onto the stack for use by the current function
    arg_size.times do |i|
        machine_push(vm, args[i])
	end
    # Free the space used to temporarily store the supplied arguments.
    # free(args)
end

# End a stack frame for a function with `return_size` number of cells
# to return, and resume the parent stack frame.
def machine_end_stack_frame(vm, return_size, local_scope_size) {
    # Allocate some space to store the returned cells for later
    return_val = new Array(return_size, 0.0)
    # Pop the returned values off of the stack
    for i in (return_size-1).downto(0)
        return_val[i] = machine_pop(vm)
	end
    # Discard the memory setup by the stack frame
	local_scope_size.times do
		machine_pop(vm)
	end
    # Retrieve the parent function's base pointer to resume the function
    vm.base_ptr = machine_pop(vm)

    # Finally, push the returned value back onto the stack for use by
    # the parent function.
    return_size.times do |i|
        machine_push(vm, return_val[i])
	end
    # Free the space used to temporarily store the returned value.
    #free(return_val)
end


####################################/
##########/ Pointer and memory operations ##########/
####################################/
# Pop the `size` parameter off of the stack, and return a pointer to `size` number of free cells.
def machine_allocate(vm) {    
    # Get the size of the memory to allocate on the heap
	size = machine_pop(vm)
	addr = 0
	consecutive_free_cells = 0

    # Starting at the end of the memory tape, find `size`
    # number of consecutive cells that have not yet been
    # allocated.
    for i in (vm.capacity-1).downto(vm.stack_ptr+1)
        # If the memory hasn't been allocated, increment the counter.
        # Otherwise, reset the counter.
		if !vm.allocated[i]
			consecutive_free_cells += 1
		else
			consecutive_free_cells = 0
		end

        # After we've found an address with the proper amount of memory left,
        # return the address.
        if consecutive_free_cells == size
            addr = i
            break
		end
    end

    # If the address is less than the stack pointer,
    # the the heap must be full.
    # The program cannot continue without undefined behavior in this state.
    if addr <= vm.stack_ptr
        panic(NO_FREE_MEMORY)
	end
    # Mark the address as allocated
    size.times do |i|
        vm.allocated[addr+i] = true
	end
    # Push the address onto the stack
    machine_push(vm, addr)
    return addr
end

# Pop the `address` and `size` parameters off of the stack, and free the memory at `address` with size `size`.
def machine_free(vm)
    # Get the address and size to free from the stack
	addr = machine_pop(vm)
	size = machine_pop(vm)

    # Mark the memory as unallocated, and zero each of the cells
    size.times do |i|
        vm.allocated[addr+i] = false
        vm.memory[addr+i] = 0
    end
end

# Pop an `address` parameter off of the stack, and a `value` parameter with size `size`.
# Then store the `value` parameter at the memory address `address`.
def machine_store(vm, size) {
    # Pop an address off of the stack
    addr = machine_pop(vm)

    # Pop `size` number of cells from the stack,
    # and store them at the address in the same order they were
    # pushed onto the stack.
	for i in (size-1).downto(0)
		vm.memory[addr+i] = machine_pop(vm)
	end
end

# Pop an `address` parameter off of the stack, and push the value at `address` with size `size` onto the stack.
def machine_load(vm, size) {
	addr = machine_pop(vm)
	size.times do |i|
		machine_push(vm, vm.memory[addr+i])
	end
end

# Add the topmost numbers on the stack
def machine_add(vm) {
    machine_push(vm, machine_pop(vm) + machine_pop(vm))
end

# Subtract the topmost number on the stack from the second topmost number on the stack
def machine_subtract(vm) {
    b = machine_pop(vm)
    a = machine_pop(vm)
    machine_push(vm, a-b)
end

# Multiply the topmost numbers on the stack
def machine_multiply(vm) {
    machine_push(vm, machine_pop(vm) * machine_pop(vm))
end

# Divide the second topmost number on the stack by the topmost number on the stack
def machine_divide(vm) {
    b = machine_pop(vm)
    a = machine_pop(vm)
    machine_push(vm, a/b)
end

def machine_sign(vm) {
    x = machine_pop(vm)
    if x >= 0
        machine_push(vm, 1)
    else
        machine_push(vm, -1)
    end
end


