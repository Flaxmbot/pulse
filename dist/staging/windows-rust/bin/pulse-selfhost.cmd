@echo off
set PULSE_COMPILER_TRACK=selfhost
set PULSE_HOME=%~dp0..
set PULSE_SELFHOST_ENTRY=%~dp0..\share\pulse\self-hosted\compiler.pulse
"%~dp0pulse_cli.exe" %*
