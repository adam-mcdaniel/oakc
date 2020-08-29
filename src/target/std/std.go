

func __oak_std__putnum(vm *machine) {
	n := vm.pop()
	fmt.Printf("%g", n)
}

func __oak_std__putstr(vm *machine) {
	addr := int(vm.pop())
	for i := addr; vm.memory[i] != 0.0; i += 1 {
		fmt.Printf("%c", rune(vm.memory[i]))
	}
}

func __oak_std__putchar(vm *machine) {
	n := vm.pop()
	fmt.Printf("%c", rune(n))
}

func __oak_std__prend(vm *machine) {
	fmt.Print("\n")
}

func __oak_std__get_char(vm *machine) {
	ch, _ := READER.ReadByte()
	if ch == '\r' {
		ch, _ = READER.ReadByte()
	}

	vm.push(float64(ch))
}

func __oak_std__get_day_now(vm *machine) {
	_, _, d := time.Now().Date()
	vm.push(float64(d))
}

func __oak_std__get_month_now(vm *machine) {
	_, m, _ := time.Now().Date()
	vm.push(float64(m-1))
}

func __oak_std__get_year_now(vm *machine) {
	y, _, _ := time.Now().Date()
	vm.push(float64(y))
}

func __oak_std__get_hour_now(vm *machine) {
	vm.push(float64(time.Now().Hour()))
}

func __oak_std__get_minute_now(vm *machine) {
	vm.push(float64(time.Now().Minute()))
}

func __oak_std__get_second_now(vm *machine) {
	vm.push(float64(time.Now().Second()))
}