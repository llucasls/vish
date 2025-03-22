# Vish  

This project aims to create a POSIX-compliant shell, following the specification outlined in:  
[POSIX Shell and Utilities Specification](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/contents.html)  

### Overview  
Vish currently supports:  
- **Command execution** (spawning child processes)  
- **Basic built-in utilities** (`exec`, `exit`, `echo`, `false`, `true`)  
- **Prompt customization** via `$PS1`  
- **Line editing improvements** (Ctrl+A, Ctrl+E, Ctrl+D)  
- **Simple parameter expansion** (`$VAR` but not `${VAR:-default}` yet)  

### Work in Progress  
- **Control flow syntax** (e.g., `if`, `while`, `for`)  
- **Full expansion support** (handling quotes, arithmetic, command substitution, etc.)  
- **More built-in utilities** (see the checklists below)  
- **Job control** (`bg`, `fg`, `jobs`, `kill`, `wait`, etc.)  
- **Pipelines** (`cmd1 | cmd2 | cmd3`)  
- **Command lists** (`cmd1 && cmd2`, `cmd1 || cmd2`, `cmd1; cmd2`)  
- **Compound commands** (`{ cmd1; cmd2; }`, `( cmd1; cmd2 )`)  
- **Function definitions** (`foo() { echo "Hello"; }`)  
- **Signal handling** (`trap`, handling `SIGINT`, `SIGTERM`, etc.)  
- **Pattern matching** (globbing: `*.txt`, `file_[0-9].log`)  

This now covers all major shell features that need to be implemented. Would you like any refinements or additions?

### Implementation Details  
Command execution follows the correct order as specified in:  
[POSIX Shell Execution Order](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/V3_chap02.html#tag_19_09_01_04)  

Unspecified utilities may or may not be implemented.  

---

## Special Built-In Utilities

- [ ] break
- [ ] colon
- [ ] continue
- [ ] dot
- [ ] eval
- [x] exec
- [x] exit
- [ ] export
- [ ] readonly
- [ ] return
- [ ] set
- [ ] shift
- [ ] times
- [ ] trap
- [ ] unset

## Built-In Utilities
- [ ] alias
- [ ] bg
- [ ] cd
- [ ] command
- [x] echo
- [x] false
- [ ] fc
- [ ] fg
- [ ] getopts
- [ ] hash
- [ ] jobs
- [ ] kill
- [ ] printf
- [ ] pwd
- [ ] read
- [ ] test
- [x] true
- [ ] type
- [ ] ulimit
- [ ] umask
- [ ] unalias
- [ ] wait
