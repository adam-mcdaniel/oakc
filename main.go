package main

import (
	"bufio"
	"fmt"
	"os"
)

var READER = bufio.NewReader(os.Stdin)

const STACK_HEAP_COLLISION = 1
const NO_FREE_MEMORY = 2
const STACK_UNDERFLOW = 3

func panic(code int) {
	fmt.Print("panic: ")
	switch code {
	case 1:
		fmt.Println("stack and heap collision during push")
		break
	case 2:
		fmt.Println("no free memory left")
		break
	case 3:
		fmt.Println("stack underflow")
		break
	default:
		fmt.Println("unknown error code")
	}
	os.Exit(code)
}

type machine struct {
	memory    []float64
	allocated []bool
	capacity  int
	base_ptr  int
	stack_ptr int
}

func machine_new(global_scope_size, capacity int) *machine {
	memory := []float64{}
	allocated := []bool{}
	for i := 0; i < capacity; i++ {
		memory = append(memory, 0)
		allocated = append(allocated, false)
	}
	result := &machine{memory, allocated, capacity, 0, 0}
	for i := 0; i < global_scope_size; i++ {
		result.push(0)
	}
	return result
}

func (vm *machine) drop() {
	// fmt.Print("stack: [ ")
	// for i:=0; i<vm.stack_ptr; i+=1 {
	// 	fmt.Printf("%g ", vm.memory[i])
	// }
	// for i:=vm.stack_ptr; i<vm.capacity; i+=1 {
	//     fmt.Print("  ")
	// }
	// fmt.Println("]")
	// fmt.Print("heap:  [ ")
	// for i:=0; i<vm.stack_ptr; i+=1 {
	// 	fmt.Print("  ")
	// }
	// for i:=vm.stack_ptr; i<vm.capacity; i+=1 {
	// 	fmt.Printf("%g ", vm.memory[i])
	// }
	// fmt.Println("]")
	// fmt.Print("alloc: [ ")
	// for i:=0; i<vm.capacity; i+=1 {
	// 	if vm.allocated[i] {
	// 		fmt.Printf("1 ")
	// 	} else {
	// 		fmt.Printf("0 ")
	// 	}
	// }
	// fmt.Println("]")
	// total := 0;
	// for i:=0; i<vm.capacity; i+=1 {
	//     if vm.allocated[i] {
	// 		total += 1
	// 	}
	// }
	// fmt.Println("STACK SIZE    %d\n", vm.stack_ptr);
	// fmt.Println("TOTAL ALLOC'D %d\n", total);
}

func (vm *machine) load_base_ptr() {
	// Get the virtual machine's current base pointer value,
	// and push it onto the stack.
	vm.push(float64(vm.base_ptr))
}

func (vm *machine) establish_stack_frame(arg_size, local_scope_size int) {
	// Allocate some space to store the arguments' cells for later
	args := make([]float64, arg_size)
	// Pop the arguments' values off of the stack
	for i := arg_size - 1; i >= 0; i -= 1 {
		args[i] = vm.pop()
	}

	// Push the current base pointer onto the stack so that
	// when this function returns, it will be able to resume
	// the current stack frame
	vm.load_base_ptr()

	// Set the base pointer to the current stack pointer to
	// begin the stack frame at the current position on the stack.
	vm.base_ptr = vm.stack_ptr

	// Allocate space for all the variables used in the local scope on the stack
	for i := 0; i < local_scope_size; i += 1 {
		vm.push(0.0)
	}

	// Push the arguments back onto the stack for use by the current function
	for i := 0; i < arg_size; i += 1 {
		vm.push(args[i])
	}
}

func (vm *machine) end_stack_frame(return_size, local_scope_size int) {
	// Allocate some space to store the returned cells for later
	return_val := make([]float64, return_size)
	// Pop the returned values off of the stack
	for i := return_size - 1; i >= 0; i -= 1 {
		return_val[i] = vm.pop()
	}

	// Discard the memory setup by the stack frame
	for i := 0; i < local_scope_size; i += 1 {
		vm.pop()
	}

	// Retrieve the parent function's base pointer to resume the function
	vm.base_ptr = int(vm.pop())

	// Finally, push the returned value back onto the stack for use by
	// the parent function.
	for i := 0; i < return_size; i += 1 {
		vm.push(return_val[i])
	}
}

func (vm *machine) push(n float64) {
	if vm.allocated[vm.stack_ptr] {
		panic(STACK_HEAP_COLLISION)
	}
	vm.memory[vm.stack_ptr] = n
	vm.stack_ptr += 1
}

func (vm *machine) pop() float64 {
	if vm.stack_ptr == 0 {
		panic(STACK_UNDERFLOW)
	}
	vm.stack_ptr -= 1
	result := vm.memory[vm.stack_ptr]
	vm.memory[vm.stack_ptr] = 0
	return result
}

func (vm *machine) allocate() int {
	size := int(vm.pop())
	addr := 0
	consecutive_free_cells := 0

	for i := vm.capacity - 1; i > vm.stack_ptr; i -= 1 {
		if !vm.allocated[i] {
			consecutive_free_cells += 1
		} else {
			consecutive_free_cells = 0
		}

		if consecutive_free_cells == size {
			addr = i
			break
		}
	}

	if addr <= vm.stack_ptr {
		panic(NO_FREE_MEMORY)
	}

	for i := 0; i < size; i += 1 {
		vm.allocated[addr+i] = true
	}

	vm.push(float64(addr))
	return addr
}

func (vm *machine) free() {
	addr := int(vm.pop())
	size := int(vm.pop())

	for i := 0; i < size; i += 1 {
		vm.allocated[addr+i] = false
		vm.memory[addr+i] = 0
	}
}

func (vm *machine) load(size int) {
	addr := int(vm.pop())
	for i := 0; i < size; i += 1 {
		vm.push(vm.memory[addr+i])
	}
}

func (vm *machine) store(size int) {
	addr := int(vm.pop())
	for i := size - 1; i >= 0; i -= 1 {
		vm.memory[addr+i] = vm.pop()
	}
}

func (vm *machine) add() {
	vm.push(vm.pop() + vm.pop())
}

func (vm *machine) subtract() {
	b := vm.pop()
	a := vm.pop()
	vm.push(a - b)
}

func (vm *machine) multiply() {
	vm.push(vm.pop() * vm.pop())
}

func (vm *machine) divide() {
	b := vm.pop()
	a := vm.pop()
	vm.push(a / b)
}

func (vm *machine) sign() {
	x := vm.pop()
	if x >= 0 {
		vm.push(1.0)
	} else {
		vm.push(-1.0)
	}
}

func prn(vm *machine) {
	n := vm.pop()
	fmt.Printf("%g", n)
}

func prs(vm *machine) {
	addr := int(vm.pop())
	for i := addr; vm.memory[i] != 0.0; i += 1 {
		fmt.Printf("%c", rune(vm.memory[i]))
	}
}

func prc(vm *machine) {
	n := vm.pop()
	fmt.Printf("%c", rune(n))
}

func prend(vm *machine) {
	fmt.Print("\n")
}

func getch(vm *machine) {
	ch, _ := READER.ReadByte()
	if ch == '\r' {
		ch, _ = READER.ReadByte()
	}

	vm.push(float64(ch))
}


func fn1(vm *machine) {
vm.establish_stack_frame(0, 1)
prend(vm);
vm.end_stack_frame(0, 1)

}


func fn2(vm *machine) {
vm.establish_stack_frame(1, 2)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.load(1)
prs(vm);
vm.end_stack_frame(0, 2)

}


func fn3(vm *machine) {
vm.establish_stack_frame(1, 2)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.load(1)
prn(vm);
vm.end_stack_frame(0, 2)

}


func fn4(vm *machine) {
vm.establish_stack_frame(1, 2)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.load(1)
prc(vm);
vm.end_stack_frame(0, 2)

}


func fn5(vm *machine) {
vm.establish_stack_frame(0, 1)
getch(vm);
vm.end_stack_frame(1, 1)

}


func fn6(vm *machine) {
vm.establish_stack_frame(1, 2)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.load(1)
fn2(vm);
fn1(vm);
vm.end_stack_frame(0, 2)

}


func fn7(vm *machine) {
vm.establish_stack_frame(1, 2)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.load(1)
fn3(vm);
fn1(vm);
vm.end_stack_frame(0, 2)

}


func fn8(vm *machine) {
vm.establish_stack_frame(1, 2)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.load(1)
fn4(vm);
fn1(vm);
vm.end_stack_frame(0, 2)

}


func fn9(vm *machine) {
vm.establish_stack_frame(1, 4)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.load(1)
vm.push(2)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(1)
vm.push(3)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(2)
vm.load_base_ptr()
vm.add()
vm.load(1)
for vm.pop() != 0.0 {
vm.push(116)
fn4(vm);
vm.push(114)
fn4(vm);
vm.push(117)
fn4(vm);
vm.push(101)
fn4(vm);
vm.push(0)
vm.push(2)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(0)
vm.push(3)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(2)
vm.load_base_ptr()
vm.add()
vm.load(1)
}
vm.push(3)
vm.load_base_ptr()
vm.add()
vm.load(1)
for vm.pop() != 0.0 {
vm.push(102)
fn4(vm);
vm.push(97)
fn4(vm);
vm.push(108)
fn4(vm);
vm.push(115)
fn4(vm);
vm.push(101)
fn4(vm);
vm.push(0)
vm.push(2)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(0)
vm.push(3)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(3)
vm.load_base_ptr()
vm.add()
vm.load(1)
}
vm.end_stack_frame(0, 4)

}


func fn10(vm *machine) {
vm.establish_stack_frame(1, 2)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.store(1)
vm.push(1)
vm.load_base_ptr()
vm.add()
vm.load(1)
fn9(vm);
fn1(vm);
vm.end_stack_frame(0, 2)

}


func fn0(vm *machine) {
vm.establish_stack_frame(0, 1)
vm.push(72)
vm.push(101)
vm.push(108)
vm.push(108)
vm.push(111)
vm.push(32)
vm.push(119)
vm.push(111)
vm.push(114)
vm.push(108)
vm.push(100)
vm.push(33)
vm.push(0)
vm.push(0)
vm.store(13)
vm.push(0)
fn6(vm);
vm.end_stack_frame(0, 1)

}
func main() {
vm := machine_new(13, 525)
fn0(vm);

vm.drop()
}