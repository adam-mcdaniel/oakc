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
	stack_ptr int
}

func machine_new(vars int, capacity int) *machine {
	memory := []float64{}
	allocated := []bool{}
	for i := 0; i < capacity; i++ {
		memory = append(memory, 0)
		allocated = append(allocated, false)
	}
	result := &machine{memory, allocated, capacity, 0}
	for i := 0; i < vars; i++ {
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
