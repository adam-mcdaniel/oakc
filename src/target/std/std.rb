
# from http://rubyquiz.com/quiz5.html
begin
	require "Win32API"

	def read_char
		Win32API.new("crtdll", "_getch", [], "L").Call
	end
rescue LoadError
	def read_char
		system "stty raw -echo"
		STDIN.getc
	ensure
		system "stty -raw echo"
	end
end

def prn(vm)
	n = machine_pop(vm)
    print(n)
end

def prs(vm)
	addr = machine_pop(vm)
	i = addr
    while vm.memory[i] != 0.0
		print(vm.memory[i].chr)
		i += 1
	end
end

def prc(vm)
    n = machine_pop(vm)
    print(n.chr)
end

def prend(vm)
    print("\n")
end

def getch(vm)
    ch = read_char
    if ch == '\r'
        ch = read_char
	end
    machine_push(vm, ch)
end

