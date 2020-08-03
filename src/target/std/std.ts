//print a number
function prn(vm: machine): void {
	let n = machine_pop(vm);
	console.log(n);
}

//print a null-terminated string
function prs(vm: machine): void {
	let addr = machine_pop(vm);
	//console.log always inserts a newline, so build the string first and then print
	let out = "";
	for (let i=addr; vm.memory[i]; i++) {
		out += String.fromCharCode(vm.memory[i]);
	}
	console.log(out);
}

//print a char
function prc(vm: machine): void {
	let n = machine_pop(vm);
	console.log(String.fromCharCode(n));
}

//print a newline
function prend(vm: machine): void {
	//console.log always inserts a newline
	console.log("");
}

async function getch(vm: machine): Promise<void> {
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
		getch(vm);
	} else {
		ch = key.charCodeAt(0);
	}
	machine_push(vm, ch);
}


