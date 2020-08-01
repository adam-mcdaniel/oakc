
void prn(machine *vm) {
    double n = machine_pop(vm);
    printf("%g", n);
}

void prs(machine *vm) {
    double addr = machine_pop(vm);
    int i;
    for (i=addr; vm->memory[i]; i++) {
        printf("%c", (char)vm->memory[i]);
    }
}

void prend(machine *vm) {
    printf("\n");
}
