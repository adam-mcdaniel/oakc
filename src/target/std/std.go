

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

func get_day_now(vm *machine) {
	_, _, d := time.Now()
	vm.push(float64(d))
}

func get_month_now(vm *machine) {
	_, m, _ := time.Now()
	vm.push(float64(m-1))
}

func get_year_now(vm *machine) {
	y, _, _ := time.Now()
	vm.push(float64(y))
}

func get_hour_now(vm *machine) {
	vm.push(float64(time.Now().Hour()))
}

func get_minute_now(vm *machine) {
	vm.push(float64(time.Now().Minute()))
}

func get_second_now(vm *machine) {
	vm.push(float64(time.Now().Second()))
}