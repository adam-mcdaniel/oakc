//print a number
function __oak_std__putnum(vm: machine): void {
	let n = machine_pop(vm);
	console.log(n);
}

//print a null-terminated string
function __oak_std__putstr(vm: machine): void {
	let addr = machine_pop(vm);
	//console.log always inserts a newline, so build the string first and then print
	let out = "";
	for (let i=addr; vm.memory[i]; i++) {
		out += String.fromCharCode(vm.memory[i]);
	}
	console.log(out);
}

//print a char
function __oak_std__putchar(vm: machine): void {
	let n = machine_pop(vm);
	console.log(String.fromCharCode(n));
}

//print a newline
function __oak_std__prend(vm: machine): void {
	//console.log always inserts a newline
	console.log("");
}

async function __oak_std__get_char(vm: machine): Promise<void> {
	//https://stackoverflow.com/questions/44746592/is-there-a-way-to-write-async-await-code-that-responds-to-onkeypress-events
	async function readKey(): Promise<KeyboardEvent>{
		return new Promise(resolve => {
			window.addEventListener('keypress', resolve, {once:true});
		});
	}
	let key: string = (await readKey()).key;
	let ch: number;

	if (key === "Enter") { //make sure pressing enter always gives \n
		ch = "\n".charCodeAt(0);
	} else if (key.length > 1){ //if the key is not a single character (arrow keys, etc.)
		//find a way to make this non-recursive
		__oak_std__get_char(vm);
	} else {
		ch = key.charCodeAt(0);
	}
	machine_push(vm, ch);
}


function __oak_std__get_day_now(vm: machine): void {
	machine_push(vm, new Date().getUTCDate())
}

function __oak_std__get_month_now(vm: machine): void {
	machine_push(vm, new Date().getMonth())
}

function __oak_std__get_year_now(vm: machine): void {
	machine_push(vm, new Date().getFullYear())
}

function __oak_std__get_hour_now(vm: machine): void {
	machine_push(vm, new Date().getHours())
}

function __oak_std__get_minute_now(vm: machine): void {
	machine_push(vm, new Date().getMinutes())
}

function __oak_std__get_second_now(vm: machine): void {
	machine_push(vm, new Date().getSeconds())
}