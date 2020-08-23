#!/usr/bin/env python3

import sys
import subprocess
import difflib
from math import floor
from typing import List, Tuple, TypeVar
StdIOType = TypeVar("StdIOType", bytes, None)

def get_longest_line_length(arr: List[str]) -> int:
	maximum = 0
	for line in arr:
		if len(line) > maximum:
			maximum = len(line)
	return maximum

def pad_lines(arr: List[str]) -> List[str]:
	length = get_longest_line_length(arr)
	return [line.ljust(length, " ") for line in arr]

def pad_list(arr: List[str], new_len: int) -> List[str]:
	out = arr.copy()
	if not len(arr) >= new_len:
		for i in range(len(arr)-1, new_len):
			out[i] = ""
	return out

def print_diff(base: bytes, test: bytes, title: str) -> None:
	base = base.decode("utf8")
	test = test.decode("utf8")
	base_lines = ["C "+title, "=========="]
	base_lines.extend(base.split("\n"))
	test_lines = ["Test "+title, "=========="]
	test_lines.extend(test.split("\n"))
	list_len = max(len(base_lines), len(test_lines))
	base_lines = pad_lines(pad_list(base_lines, list_len))
	test_lines = pad_lines(pad_list(test_lines, list_len))
	arrow_pos = ["     "]*list_len
	
	for i,s in enumerate(difflib.ndiff(base, test)):
		if s[0]==' ': continue
		elif s[0]=='-' or s[0]=='+':
			arrow_pos[base[:i].count("\n")+2] = " =/= "
	for i in range(0, list_len):
		print("| "+base_lines[i]+" | "+arrow_pos[i]+" | "+test_lines[i]+" |")

def run_and_capture_output(args: List) -> Tuple[bytes, bytes]:
	complete_process = subprocess.Popen(args,
	                   stdout=subprocess.PIPE,
					   stderr=subprocess.STDOUT)
	return complete_process.communicate()

def verbose_process_output(stdout: StdIOType, stderr: StdIOType, verbose: bool) -> None:
	if verbose and not stderr:
		print(stdout.decode("utf-8"))
	elif verbose:
		print(stderr.decode("utf-8"))

def main():
	backend_to_test = sys.argv[sys.argv.index("-b")+1]
	run_cmd = sys.argv[sys.argv.index("-r")+1]
	file_to_test = sys.argv[sys.argv.index("-f")+1]
	verbose = False
	try:
		sys.argv.index("-v")
		verbose = True
	except: 
		pass
		
	if verbose:
		print("Compiling "+file_to_test+" with C backend...")
	baseline_compile_stdout, baseline_compile_stderr = run_and_capture_output(
		["./target/debug/oak", "-c", "c", file_to_test]
	)
	verbose_process_output(baseline_compile_stdout, baseline_compile_stderr, verbose)

	if verbose:
		print("Running "+file_to_test+" with C backend...")
	baseline_run_stdout, baseline_run_stderr = run_and_capture_output(["./main"])
	verbose_process_output(baseline_run_stdout, baseline_run_stderr, verbose)

	if verbose:
		print("Compiling "+file_to_test+" with test backend...")
	test_compile_stdout, test_compile_stderr = run_and_capture_output(
		["./target/debug/oak", backend_to_test, "c", file_to_test]
	)
	verbose_process_output(test_compile_stdout, test_compile_stderr, verbose)

	if verbose:
		print("Running "+file_to_test+" with test backend...")
	test_run_stdout, test_run_stderr = run_and_capture_output(run_cmd.split(" "))
	verbose_process_output(test_run_stdout, test_run_stderr, verbose)

	try:
		assert(baseline_compile_stdout == test_compile_stdout)
	except: 
		print("Test Failed!")
		print_diff(baseline_compile_stdout, test_compile_stdout, "compile_stdout")
	try:
		assert(baseline_compile_stderr == test_compile_stderr)
	except: 
		print("Test Failed!")
		print_diff(baseline_compile_stderr, test_compile_stderr, "compile_stderr")
	try:
		assert(baseline_run_stdout == test_run_stdout)
	except: 
		print("Test Failed!")
		print_diff(baseline_run_stdout, test_run_stdout, "run_stdout")
	try:
		assert(baseline_run_stderr == test_run_stderr)
	except: 
		print("Test Failed!")
		print_diff(baseline_run_stderr, test_run_stderr, "run_stderr")

if __name__ == "__main__":
	main()