
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

#include <time.h>

void get_day_now(machine *vm) {
    struct tm *t = localtime(time(NULL));
    machine_push(vm, t->tm_mday);
}

void get_month_now(machine *vm) {
    struct tm *t = localtime(time(NULL));
    machine_push(vm, t->tm_mon);
}

void get_year_now(machine *vm) {
    struct tm *t = localtime(time(NULL));
    machine_push(vm, t->tm_year + 1900);
}

void get_hour_now(machine *vm) {
    struct tm *t = localtime(time(NULL));
    machine_push(vm, t->tm_hour);
}

void get_minute_now(machine *vm) {
    struct tm *t = localtime(time(NULL));
    machine_push(vm, t->tm_min);
}

void get_second_now(machine *vm) {
    struct tm *t = localtime(time(NULL));
    machine_push(vm, t->tm_sec);
}