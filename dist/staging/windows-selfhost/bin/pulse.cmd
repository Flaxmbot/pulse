@echo off
set PULSE_COMPILER_TRACK=selfhost
set PULSE_HOME=%~dp0..
"%~dp0pulse_cli.exe" %*
