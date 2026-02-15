
lines = open("pulse_vm/src/vm.rs", "r").readlines()
for i, line in enumerate(lines):
    if "self.frames.push(frame);" in line and i > 1250:
        print(f"Line {i+1}: '{line}'")
