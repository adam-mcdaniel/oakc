
void __oak_std__putnum(machine *vm) {
    double n = machine_pop(vm);
    printf("%g", n);
}

void __oak_std__putstr(machine *vm) {
    double addr = machine_pop(vm);
    int i;
    for (i=addr; vm->memory[i]; i++) {
        printf("%c", (char)vm->memory[i]);
    }
}

void __oak_std__putchar(machine *vm) {
    double n = machine_pop(vm);
    printf("%c", (char)n);
}

void __oak_std__prend(machine *vm) {
    printf("\n");
}

void __oak_std__get_char(machine *vm) {
    char ch = getchar();
    if (ch == '\r') {
        ch = getchar();
    }
    machine_push(vm, ch);
}

#include <time.h>

time_t epoch = 0;

void __oak_std__get_day_now(machine *vm) {
    time(&epoch);
    struct tm *t = localtime(&epoch);
    machine_push(vm, t->tm_mday);
}

void __oak_std__get_month_now(machine *vm) {
    time(&epoch);
    struct tm *t = localtime(&epoch);
    machine_push(vm, t->tm_mon);
}

void __oak_std__get_year_now(machine *vm) {
    time(&epoch);
    struct tm *t = localtime(&epoch);
    machine_push(vm, t->tm_year + 1900);
}

void __oak_std__get_hour_now(machine *vm) {
    time(&epoch);
    struct tm *t = localtime(&epoch);
    machine_push(vm, t->tm_hour);
}

void __oak_std__get_minute_now(machine *vm) {
    time(&epoch);
    struct tm *t = localtime(&epoch);
    machine_push(vm, t->tm_min);
}

void __oak_std__get_second_now(machine *vm) {
    time(&epoch);
    struct tm *t = localtime(&epoch);
    machine_push(vm, t->tm_sec);
}