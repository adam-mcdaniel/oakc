#include "oak.h"

void fn0(machine *vm);
void fn1(machine *vm);
void fn2(machine *vm);
void fn3(machine *vm);
void fn4(machine *vm);
void fn0(machine *vm) {
machine_push(vm, 0);machine_store(vm, 1);
machine_push(vm, 1);
machine_push(vm, 1);machine_store(vm, 1);
machine_push(vm, 0);
machine_load(vm, 1);
machine_push(vm, 2);machine_store(vm, 1);
machine_push(vm, 2);
machine_load(vm, 1);
while (machine_pop(vm)) {machine_push(vm, 0);
machine_push(vm, 1);
machine_store(vm, 1);
machine_push(vm, 0);
machine_push(vm, 2);
machine_store(vm, 1);
machine_push(vm, 2);
machine_load(vm, 1);

}
machine_push(vm, 1);
machine_load(vm, 1);
}

void fn1(machine *vm) {
machine_push(vm, 3);machine_store(vm, 1);
machine_push(vm, 4);machine_store(vm, 1);
machine_push(vm, 4);
machine_load(vm, 1);
machine_push(vm, 3);
machine_load(vm, 1);
fn2(vm);
fn0(vm);
}

void fn2(machine *vm) {
machine_push(vm, 5);machine_store(vm, 1);
machine_push(vm, 6);machine_store(vm, 1);
machine_push(vm, 1);
machine_push(vm, 7);machine_store(vm, 1);
machine_push(vm, 5);
machine_load(vm, 1);
machine_push(vm, 6);
machine_load(vm, 1);
machine_subtract(vm);
machine_push(vm, 8);machine_store(vm, 1);
machine_push(vm, 8);
machine_load(vm, 1);
while (machine_pop(vm)) {machine_push(vm, 0);
machine_push(vm, 7);
machine_store(vm, 1);
machine_push(vm, 0);
machine_push(vm, 8);
machine_store(vm, 1);
machine_push(vm, 8);
machine_load(vm, 1);

}
machine_push(vm, 7);
machine_load(vm, 1);
}

void fn3(machine *vm) {
machine_push(vm, 9);machine_store(vm, 1);
machine_push(vm, 1);
machine_push(vm, 9);
machine_load(vm, 1);
fn2(vm);
machine_push(vm, 10);machine_store(vm, 1);
machine_push(vm, 10);
machine_load(vm, 1);
while (machine_pop(vm)) {machine_push(vm, 1);
machine_push(vm, 0);
machine_push(vm, 10);
machine_store(vm, 1);
machine_push(vm, 10);
machine_load(vm, 1);

}
machine_push(vm, 1);
machine_push(vm, 9);
machine_load(vm, 1);
fn1(vm);
machine_push(vm, 11);machine_store(vm, 1);
machine_push(vm, 11);
machine_load(vm, 1);
while (machine_pop(vm)) {machine_push(vm, 9);
machine_load(vm, 1);
machine_push(vm, 9);
machine_load(vm, 1);
machine_push(vm, 1);
machine_subtract(vm);
fn3(vm);
machine_multiply(vm);
machine_push(vm, 0);
machine_push(vm, 11);
machine_store(vm, 1);
machine_push(vm, 11);
machine_load(vm, 1);

}
}

void fn4(machine *vm) {
machine_push(vm, 5);
fn3(vm);
prn(vm);
prend(vm);
}

int main() {
machine *vm = machine_new(12, 524);
fn4(vm);
machine_drop(vm);
return 0;
}