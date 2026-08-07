import subprocess
import threading
import sys
import os
import re
import time

try:
    from universal_generator import generate_universal
except ImportError:
    print("[ERROR] File 'universal_generator.py' not found!")
    sys.exit(1)

# Find the path to the Rust node.exe application
exe_path = os.path.join("..", "coiwin-node-windows", "node.exe")
if not os.path.exists(exe_path):
    print(f"[ERROR] Executable node.exe not found at: {exe_path}")
    print("Ensure the '! Coywin' and 'coiwin-node-windows' folders are side-by-side.")
    sys.exit(1)

print("="*50)
print("       COYWIN MANUAL MINER (BRIDGE)        ")
print("="*50)
print(f"Node Connected: {exe_path}")
print("Interactive mode is active. You can control the Node manually from here.")
print("Type 'mine' then Enter to start searching for blocks.")
print("Type 'exit' to quit.")
print("-" * 50)

# Run the Rust node.exe process
process = subprocess.Popen(
    [exe_path],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.STDOUT, 
    text=True,
    bufsize=1
)

# Intelligent regex to detect 64-character hash (SHA-256)
hash_pattern = re.compile(r'\b([a-fA-F0-9]{64})\b')

def read_output():
    buffer = ""
    while True:
        try:
            char = process.stdout.read(1)
            if not char:
                break
            
            # Print instantly for smooth output
            print(char, end="", flush=True)
            buffer += char
            
            if char in (' ', '\n', '\t'):
                # Scan if this line contains a valid Hash
                matches = hash_pattern.findall(buffer)
                for h in matches:
                    print(f"\n[BRIDGE] ⚡ VALID BLOCK HASH DETECTED: {h}")
                    print(f"[BRIDGE] 🎨 Activating Coywin Canvas to paint...")
                    try:
                        generate_universal(h)
                    except Exception as e:
                        print(f"[BRIDGE ERROR] Failed to render: {e}")
                buffer = ""
        except Exception as e:
            break

# Run output capturer in the background
thread = threading.Thread(target=read_output, daemon=True)
thread.start()

# Manual input loop
try:
    time.sleep(2) # Give time for the Node to finish its initial loading
    while process.poll() is None:
        cmd = input()
        if cmd.strip().lower() == 'exit':
            break
        process.stdin.write(cmd + "\n")
        process.stdin.flush()
except KeyboardInterrupt:
    pass

print("\nShutting down Bridge system...")
process.terminate()
sys.exit(0)
