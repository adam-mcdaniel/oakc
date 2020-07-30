
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

void prc(machine *vm) {
    double n = machine_pop(vm);
    printf("%c", (char)n);
}

void prend(machine *vm) {
    printf("\n");
}

void getch(machine *vm) {
    char ch = getchar();
    if (ch == '\r') {
        ch = getchar();
    }
    machine_push(vm, ch);
}

