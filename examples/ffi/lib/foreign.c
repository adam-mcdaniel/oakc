

void test(machine *vm) {
    printf("This is a C foreign function!\n");
}

void __oak_add(machine *vm) {
    int a = machine_pop(vm);
    int b = machine_pop(vm);
    printf("This should print %d => ", a + b);
    machine_push(vm, a + b);
}