import customtkinter as ctk
from PIL import Image
import threading
import subprocess
import os
import re
import sys
import tkinter.filedialog as filedialog
from tkinter import messagebox

try:
    from universal_generator import generate_universal
except ImportError:
    pass

try:
    from coywin_watermark import CoywinWatermark
except ImportError:
    CoywinWatermark = None

# Configure CustomTkinter
ctk.set_appearance_mode("dark")
ctk.set_default_color_theme("green") # Using green as base, we'll customize to Cyan/Gold

class CoywinDesktop(ctk.CTk):
    def __init__(self):
        super().__init__()

        self.title("Coywin - PQC Decentralized Generative Art Network")
        self.geometry("1000x700")
        
        # Grid Layout
        self.grid_rowconfigure(0, weight=1)
        self.grid_columnconfigure(1, weight=1)

        # ---------------- SIDEBAR ---------------- #
        self.sidebar_frame = ctk.CTkFrame(self, width=200, corner_radius=0)
        self.sidebar_frame.grid(row=0, column=0, sticky="nsew")
        self.sidebar_frame.grid_rowconfigure(4, weight=1)

        self.logo_label = ctk.CTkLabel(self.sidebar_frame, text="COYWIN", font=ctk.CTkFont(size=24, weight="bold"), text_color="#64ffda")
        self.logo_label.grid(row=0, column=0, padx=20, pady=(20, 10))
        
        self.subtitle = ctk.CTkLabel(self.sidebar_frame, text="Proof of Work Gallery", font=ctk.CTkFont(size=12), text_color="#8892b0")
        self.subtitle.grid(row=1, column=0, padx=20, pady=(0, 20))

        self.btn_miner = ctk.CTkButton(self.sidebar_frame, text="Miner Dashboard", command=self.show_miner_tab, fg_color="transparent", border_width=2, text_color=("gray10", "#DCE4EE"))
        self.btn_miner.grid(row=2, column=0, padx=20, pady=10)

        self.btn_watermark = ctk.CTkButton(self.sidebar_frame, text="Watermark Studio", command=self.show_watermark_tab, fg_color="transparent", border_width=2, text_color=("gray10", "#DCE4EE"))
        self.btn_watermark.grid(row=3, column=0, padx=20, pady=10)

        # ---------------- MAIN CONTENT ---------------- #
        self.main_container = ctk.CTkFrame(self, corner_radius=10, fg_color="transparent")
        self.main_container.grid(row=0, column=1, padx=20, pady=20, sticky="nsew")
        self.main_container.grid_rowconfigure(0, weight=1)
        self.main_container.grid_columnconfigure(0, weight=1)

        # Tabs Setup
        self.miner_frame = self.create_miner_frame()
        self.watermark_frame = self.create_watermark_frame()

        # Start Node Process
        self.node_process = None
        self.is_mining = False
        
        self.show_miner_tab()
        self.start_node_bridge()

    def show_miner_tab(self):
        self.watermark_frame.grid_forget()
        self.miner_frame.grid(row=0, column=0, sticky="nsew")
        self.btn_miner.configure(fg_color="#64ffda", text_color="black")
        self.btn_watermark.configure(fg_color="transparent", text_color="#DCE4EE")

    def show_watermark_tab(self):
        self.miner_frame.grid_forget()
        self.watermark_frame.grid(row=0, column=0, sticky="nsew")
        self.btn_watermark.configure(fg_color="#f9d342", text_color="black")
        self.btn_miner.configure(fg_color="transparent", text_color="#DCE4EE")

    # ================== MINER DASHBOARD ================== #
    def create_miner_frame(self):
        frame = ctk.CTkFrame(self.main_container, fg_color="transparent")
        frame.grid_rowconfigure(1, weight=1)
        frame.grid_columnconfigure(0, weight=1)
        frame.grid_columnconfigure(1, weight=1)

        # Top Control
        top_bar = ctk.CTkFrame(frame, fg_color="#1e1e1e", corner_radius=10)
        top_bar.grid(row=0, column=0, columnspan=2, sticky="ew", pady=(0, 20))
        
        title = ctk.CTkLabel(top_bar, text="Manual Miner Bridge", font=ctk.CTkFont(size=18, weight="bold"))
        title.pack(side="left", padx=20, pady=15)

        self.btn_engage = ctk.CTkButton(top_bar, text="▶ ENGAGE MINER (1-CLICK)", command=self.trigger_mine, font=ctk.CTkFont(weight="bold"), fg_color="#f9d342", text_color="black", hover_color="#c8a834")
        self.btn_engage.pack(side="right", padx=20, pady=15)

        # Terminal Log
        self.terminal = ctk.CTkTextbox(frame, fg_color="#0a0a0f", text_color="#00ff00", font=("Consolas", 12))
        self.terminal.grid(row=1, column=0, sticky="nsew", padx=(0, 10))
        self.terminal.insert("end", "Coywin GUI Bridge Initialized.\nWaiting for Node connection...\n")

        # Art Preview
        self.preview_frame = ctk.CTkFrame(frame, fg_color="#1e1e1e")
        self.preview_frame.grid(row=1, column=1, sticky="nsew")
        self.preview_frame.grid_rowconfigure(0, weight=1)
        self.preview_frame.grid_columnconfigure(0, weight=1)

        self.img_label = ctk.CTkLabel(self.preview_frame, text="NO BLOCK DISCOVERED YET", text_color="#8892b0")
        self.img_label.grid(row=0, column=0)

        return frame

    # ================== WATERMARK STUDIO ================== #
    def create_watermark_frame(self):
        frame = ctk.CTkFrame(self.main_container, fg_color="transparent")
        frame.grid_rowconfigure(2, weight=1)
        frame.grid_columnconfigure(0, weight=1)
        frame.grid_columnconfigure(1, weight=1)

        title = ctk.CTkLabel(frame, text="Watermark Auto-Verification Studio", font=ctk.CTkFont(size=22, weight="bold"), text_color="#f9d342")
        title.grid(row=0, column=0, columnspan=2, pady=(0, 20))

        # Extract Panel
        ext_panel = ctk.CTkFrame(frame)
        ext_panel.grid(row=1, column=0, columnspan=2, sticky="nsew", padx=100, pady=10)

        ctk.CTkLabel(ext_panel, text="Verify Artwork Authenticity", font=ctk.CTkFont(weight="bold")).pack(pady=20)
        self.ext_img_path = ctk.StringVar()
        ctk.CTkButton(ext_panel, text="Select Image to Verify", command=lambda: self.select_file(self.ext_img_path)).pack(pady=10)
        ctk.CTkLabel(ext_panel, textvariable=self.ext_img_path, font=("Arial", 10)).pack()

        ctk.CTkButton(ext_panel, text="🔓 Auto-Extract & Verify", fg_color="#f9d342", text_color="black", command=self.run_extract).pack(pady=30)

        # Output Log
        self.wm_log = ctk.CTkTextbox(frame, height=150, fg_color="#0a0a0f", text_color="white")
        self.wm_log.grid(row=2, column=0, columnspan=2, sticky="nsew", padx=10, pady=10)

        return frame

    def select_file(self, var):
        path = filedialog.askopenfilename(filetypes=[("PNG Images", "*.png")])
        if path:
            var.set(path)

    def log_wm(self, text):
        self.wm_log.insert("end", text + "\n")
        self.wm_log.see("end")

    def get_miner_key(self):
        import json
        wallet_path = "miner_wallet.json"
        if not os.path.exists(wallet_path):
            wallet_path = os.path.join("..", "coiwin-node-windows", "miner_wallet.json")
        if os.path.exists(wallet_path):
            try:
                with open(wallet_path, 'r') as f:
                    wallet = json.load(f)
                return wallet.get('ecdsa_secret', None)
            except Exception:
                return None
        return None

    def run_extract(self):
        if not CoywinWatermark:
            return
        p = self.ext_img_path.get()
        if not p:
            self.log_wm("[-] Please select an image to verify.")
            return
            
        k = self.get_miner_key()
        if not k:
            self.log_wm("[-] FATAL: Cannot auto-extract. miner_wallet.json not found or invalid.")
            return
        
        self.log_wm(f"[*] Analyzing {os.path.basename(p)}...")
        try:
            cw = CoywinWatermark(k)
            found = cw.extract(p)
            if found:
                self.log_wm("[+] WATERMARK MATCH: " + ", ".join(found))
            else:
                self.log_wm("[-] NO valid watermark found with this key.")
        except Exception as e:
            self.log_wm(f"[!] Error: {str(e)}")

    # ================== MINER LOGIC ================== #
    def log_terminal(self, text):
        self.terminal.insert("end", text)
        self.terminal.see("end")

    def trigger_mine(self):
        if not self.node_process or self.node_process.poll() is not None:
            self.log_terminal("\n[GUI ERROR] Node is not running!\n")
            return
        
        if self.is_mining:
            self.log_terminal("\n[GUI] Already mining. Please wait for this session to end.\n")
            return

        self.is_mining = True
        self.btn_engage.configure(state="disabled", fg_color="gray")
        self.log_terminal("\n[GUI] Executing 1-Click Mining Session...\n")
        
        try:
            self.node_process.stdin.write("mine\n")
            self.node_process.stdin.flush()
        except Exception as e:
            self.log_terminal(f"\n[GUI ERROR] Failed to send command: {e}\n")
            self.btn_engage.configure(state="normal", fg_color="#f9d342")
            self.is_mining = False

    def start_node_bridge(self):
        import ctypes
        kernel32 = ctypes.windll.kernel32
        user32 = ctypes.windll.user32
        
        # Allocate a hidden console so the child process inherits a UTF-8 console environment
        # This prevents UnicodeEncodeError when the GUI is compiled with --noconsole
        kernel32.AllocConsole()
        hwnd = kernel32.GetConsoleWindow()
        if hwnd:
            user32.ShowWindow(hwnd, 0) # SW_HIDE
            
        kernel32.SetConsoleOutputCP(65001)
        kernel32.SetConsoleCP(65001)

        exe_path = "node.exe"
        if not os.path.exists(exe_path):
            exe_path = os.path.join("..", "coiwin-node-windows", "node.exe")
            
        if not os.path.exists(exe_path):
            self.log_terminal(f"\n[FATAL] Executable node.exe not found at: {exe_path}\n")
            return
        
        env = dict(os.environ)
        env["PYTHONIOENCODING"] = "utf-8"
        env["PYTHONUTF8"] = "1"
        
        self.node_process = subprocess.Popen(
            [exe_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT, 
            text=True,
            encoding="utf-8",
            errors="replace",
            bufsize=1,
            env=env
        )
        
        thread = threading.Thread(target=self.read_node_output, daemon=True)
        thread.start()

    def load_art_preview(self, filepath):
        if os.path.exists(filepath):
            img = Image.open(filepath)
            img.thumbnail((400, 400)) # Resize for UI
            ctk_img = ctk.CTkImage(light_image=img, dark_image=img, size=img.size)
            self.img_label.configure(image=ctk_img, text="")
        else:
            self.log_terminal(f"\n[GUI ERROR] Image not found: {filepath}\n")

    def read_node_output(self):
        hash_pattern = re.compile(r'\b([a-fA-F0-9]{64})\b')
        file_pattern = re.compile(r'File\s*:\s*(.*\.png)')
        buffer = ""
        
        while True:
            try:
                char = self.node_process.stdout.read(1)
                if not char:
                    break
                
                # Update UI safely
                self.after(0, self.log_terminal, char)
                buffer += char
                
                if char in (' ', '\n', '\t'):
                    # 1. Detect Hash
                    matches = hash_pattern.findall(buffer)
                    for h in matches:
                        self.after(0, self.log_terminal, f"\n[BRIDGE] ⚡ VALID BLOCK HASH DETECTED: {h}\n[BRIDGE] 🎨 Activating Coywin Canvas to paint...\n")
                        try:
                            # Generate universal art
                            # generate_universal will print to stdout, which will go to the normal console, 
                            # not the node's stdout. So we intercept it or run it.
                            generate_universal(h)
                            
                            # Because this is a single session, reset the button state
                            self.is_mining = False
                            self.after(0, lambda: self.btn_engage.configure(state="normal", fg_color="#f9d342"))
                        except Exception as e:
                            self.after(0, self.log_terminal, f"\n[BRIDGE ERROR] Failed to render: {e}\n")
                            self.is_mining = False
                            self.after(0, lambda: self.btn_engage.configure(state="normal", fg_color="#f9d342"))
                    
                    # 2. Detect Generated Filename to load in preview
                    file_matches = file_pattern.findall(buffer)
                    for f in file_matches:
                        clean_path = f.strip()
                        self.after(0, self.load_art_preview, clean_path)

                    buffer = ""
            except Exception as e:
                break

if __name__ == "__main__":
    app = CoywinDesktop()
    app.mainloop()
