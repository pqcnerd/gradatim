#!/bin/bash
# Shell script wrapper - just calls the batch file via cmd.exe
cd "$(dirname "$0")" || exit 1
cmd.exe //c build.bat "$@"
