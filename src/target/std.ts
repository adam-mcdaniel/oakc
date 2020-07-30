interface machine {
	memory: number[];
	allocated: boolean[];
	capacity: number;
	stack_ptr: number;
}

const OAK_TS__PSEUDO_CONSOLE = document.getElementById("oak-out");

function OAK_TS__PRINT(x: any){
	if(OAK_TS__PSEUDO_CONSOLE) OAK_TS__PSEUDO_CONSOLE.innerHTML += x;
	else console.log(x);
}

///////////////////////////////////////////////////////////////////////
///////////////////////////// Error codes /////////////////////////////
///////////////////////////////////////////////////////////////////////
const STACK_HEAP_COLLISION : number = 1;
const NO_FREE_MEMORY : number	    = 2;
const STACK_UNDERFLOW : number	    = 3;

// Fatal error handler. Always exits program.
function panic(code: number): void {
	let message: string = "";
	message += "panic: ";
	switch (code) {
		case 1: message += "stack and heap collision during push"; break;
		case 2: message += "no free memory left"; break;
		case 3: message += "stack underflow"; break;
		default: message += "unknown error code";
	}
	message += "\n";
	throw new Error(message);
}

// Create new virtual machine
function machine_new(vars: number, capacity: number): machine {
	let result: machine = {
		capacity: capacity,
		memory: Array<number>(capacity),
		allocated: Array<boolean>(capacity),
		stack_ptr: 0
	}
	
	for (let i = 0; i < capacity; i++) {
		result.memory[i] = 0;
		result.allocated[i] = false;
	}

	for (let i = 0; i < vars; i++)
		machine_push(result, 0);

	return result;
}

// Free the virtual machine's memory. This is called at the end of the program.
function machine_drop(vm: machine): void {
	//let i:number;
	//OAK_TS__PRINT("stack: [ ");
	//for (i=0; i<vm.stack_ptr; i++)
	//	OAK_TS__PRINT(vm.memory[i]);
	//for (i=vm.stack_ptr; i<vm.capacity; i++)
	//	OAK_TS__PRINT("  ");
	//OAK_TS__PRINT("]\nheap:  [ ");
	//for (i=0; i<vm.stack_ptr; i++)
	//	OAK_TS__PRINT("  ");
	//for (i=vm.stack_ptr; i<vm.capacity; i++)
	//	OAK_TS__PRINT(`${vm.memory[i]} `);
	//OAK_TS__PRINT("]\nalloc: [ ");
	//for (i=0; i<vm.capacity; i++)
	//	OAK_TS__PRINT(`${vm.allocated[i]} `);
	//OAK_TS__PRINT("]\n");
	//let total: number = 0;
	//for (i=0; i<vm.capacity; i++)
	//	total += vm.allocated[i] ? 1 : 0;
	//OAK_TS__PRINT(`STACK SIZE	${vm.stack_ptr}\n`);
	//OAK_TS__PRINT(`TOTAL ALLOC'D ${total}\n`);

	//free(vm.memory);
	//free(vm.allocated);
}

// Push a number onto the stack
function machine_push(vm: machine, n: number): void {
	if (vm.allocated[vm.stack_ptr])
		panic(STACK_HEAP_COLLISION);
	vm.memory[vm.stack_ptr++] = n;
}

// Pop a number from the stack
function machine_pop(vm: machine): number {
	if (vm.stack_ptr == 0) {
		panic(STACK_UNDERFLOW);
	}
	let result: number = vm.memory[vm.stack_ptr-1];
	vm.memory[--vm.stack_ptr] = 0;
	//--vm.stack_ptr;
	return result;
}

// Pop the `size` parameter off of the stack, and return a pointer to `size` number of free cells.
function machine_allocate(vm: machine): number {	
	let size = machine_pop(vm);
	let addr = 0;
	let consecutive_free_cells = 0;

	for (let i = vm.capacity-1; i > vm.stack_ptr; i--) {
		if (!vm.allocated[i]) consecutive_free_cells++;
		else consecutive_free_cells = 0;

		if (consecutive_free_cells == size) {
			addr = i;
			break;
		}
	}

	if (addr <= vm.stack_ptr)
		panic(NO_FREE_MEMORY);
	
	for (let i = 0; i < size; i++)
		vm.allocated[addr+i] = true;

	machine_push(vm, addr);
	return addr;
}

// Pop the `address` and `size` parameters off of the stack, and free the memory at `address` with size `size`.
function machine_free(vm: machine): void {
	let addr = machine_pop(vm);
	let size = machine_pop(vm);

	for (let i=0; i<size; i++) {
		vm.allocated[addr+i] = false;
		vm.memory[addr+i] = 0;
	}
}

// Pop an `address` parameter off of the stack, and a `value` parameter with size `size`.
// Then store the `value` parameter at the memory address `address`.
function machine_store(vm: machine, size: number): void {
	let addr = machine_pop(vm);

	for (let i = size-1; i >= 0; i--) vm.memory[addr+i] = machine_pop(vm);
}

// Pop an `address` parameter off of the stack, and push the value at `address` with size `size` onto the stack.
function machine_load(vm: machine, size: number): void {
	let addr = machine_pop(vm);

	for (let i=0; i<size; i++) machine_push(vm, vm.memory[addr+i]);
}

// Add the topmost numbers on the stack
function machine_add(vm: machine): void {
	machine_push(vm, machine_pop(vm) + machine_pop(vm));
}

// Subtract the topmost number on the stack from the second topmost number on the stack
function machine_subtract(vm: machine): void {
	let b = machine_pop(vm);
	let a = machine_pop(vm);
	machine_push(vm, a-b);
}

// Multiply the topmost numbers on the stack
function machine_multiply(vm: machine): void {
	machine_push(vm, machine_pop(vm) * machine_pop(vm));
}

// Divide the second topmost number on the stack by the topmost number on the stack
function machine_divide(vm: machine): void {
	let b = machine_pop(vm);
	let a = machine_pop(vm);
	machine_push(vm, a/b);
}

function prn(vm: machine): void {
	let n = machine_pop(vm);
	OAK_TS__PRINT(n);
	
}

function prs(vm: machine): void {
	let addr = machine_pop(vm);
	for (let i=addr; vm.memory[i]; i++) {
		OAK_TS__PRINT(String.fromCharCode(vm.memory[i]));
	}
}

function prc(vm: machine): void {
	let n = machine_pop(vm);
	OAK_TS__PRINT(String.fromCharCode(n));
}

function prend(vm: machine): void {
	OAK_TS__PRINT("<br />");
}

async function getch(vm: machine): Promise<void> {
	//doesn't work
	//https://stackoverflow.com/questions/44746592/is-there-a-way-to-write-async-await-code-that-responds-to-onkeypress-events
	async function readKey(): Promise<KeyboardEvent>{
		return new Promise(resolve => {
			window.addEventListener('keypress', resolve, {once:true});
		});
	}
	let key: string = (await readKey()).key;
	let ch: number;
	//if (ch === '\r'.charCodeAt(0)) {
	//	key = (await readKey()).key;
	//	ch = key.charCodeAt(0);
	//}

	if (key === "Enter") {
		ch = "\n".charCodeAt(0);
	} else if (key.length > 1){
		//find a way to make this non-recursive
		getch(vm);
	} else {
		ch = key.charCodeAt(0);
	}
	machine_push(vm, ch);
}

function gt(vm: machine): void {
	machine_push(vm, machine_pop(vm)>machine_pop(vm)? 1 : 0);
}

function ge(vm: machine): void {
	machine_push(vm, machine_pop(vm)>=machine_pop(vm)? 1 : 0);
}

function lt(vm: machine): void {
	machine_push(vm, machine_pop(vm)<machine_pop(vm)? 1 : 0);
}

function le(vm: machine): void {
	machine_push(vm, machine_pop(vm)<=machine_pop(vm)? 1 : 0);
}

