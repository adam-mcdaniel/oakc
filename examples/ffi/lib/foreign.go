
func test(vm *machine) {
	fmt.Println("This is a Go foreign function!")
}

func __oak_add(vm *machine) {
	a := vm.pop()
	b := vm.pop()
	fmt.Printf("This should print %v => ", a+b)
	vm.push(float64(a + b))
}