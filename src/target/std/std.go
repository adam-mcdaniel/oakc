
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
