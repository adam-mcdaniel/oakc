# tests

This directory contains tests for the compiler.

### compare.py

This script allows you to test the output of an Oak program compiled with the C backend against the output of the same program compiled with any other backend.

```
Flags:
    -b: the backend to be tested (ex. "--cc", "--go")
        best to use the "--" version of the flag to avoid clashing with 
        this program's flags
    -r: the command to run the output of the test backend (ex. "./main")
    -f: the file to be tested (ex. "./examples/num.ok")
    -v: verbose output, optional
```
