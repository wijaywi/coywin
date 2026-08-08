import os
import sys
import hashlib
import numpy as np
from PIL import Image

def sieve_of_eratosthenes(limit):
    """Generate a list of prime numbers up to a given limit."""
    is_prime = [True] * (limit + 1)
    p = 2
    while (p * p <= limit):
        if is_prime[p]:
            for i in range(p * p, limit + 1, p):
                is_prime[i] = False
        p += 1
    
    prime_numbers = []
    for p in range(2, limit):
        if is_prime[p]:
            prime_numbers.append(p)
    return prime_numbers

def text_to_bits(text):
    """Convert text to a list of bits (0 and 1) with a Null Terminator."""
    bits = []
    for char in text:
        bin_str = format(ord(char), '08b')
        for bit in bin_str:
            bits.append(int(bit))
    bits.extend([0] * 8)
    return bits

def bits_to_text(bits):
    """Convert a list of bits back to text, stopping at the Null Terminator."""
    chars = []
    for i in range(0, len(bits), 8):
        byte = bits[i:i+8]
        if len(byte) < 8:
            break
        if sum(byte) == 0:
            break
        byte_str = ''.join(str(b) for b in byte)
        char = chr(int(byte_str, 2))
        chars.append(char)
    return ''.join(chars)

def generate_keystream(private_key, length):
    """Generate a deterministic pseudo-random keystream from the Private-Key hash."""
    keystream = []
    current_hash = hashlib.sha256(private_key.encode('utf-8')).digest()
    while len(keystream) < length:
        for byte in current_hash:
            bin_str = format(byte, '08b')
            for bit in bin_str:
                keystream.append(int(bit))
                if len(keystream) == length:
                    break
            if len(keystream) == length:
                break
        current_hash = hashlib.sha256(current_hash).digest()
    return keystream

def encrypt_decrypt_bits(bits, private_key):
    """Encrypt or decrypt bits using XOR (Stream Cipher)."""
    keystream = generate_keystream(private_key, len(bits))
    return [b ^ k for b, k in zip(bits, keystream)]

class CoywinWatermark:
    def __init__(self, private_key):
        self.private_key = private_key
        # We use a max block limit of 500x500, but realization is dynamic
        self.max_block_size = 500

    def embed(self, image_path, output_path):
        filename = os.path.basename(image_path)
        name_no_ext = os.path.splitext(filename)[0]
        dynamic_part = name_no_ext[:4]
        
        payload_text = f"coywin_{dynamic_part}"
        payload_bits = text_to_bits(payload_text)
        
        # 1. Encrypt the payload bits using Private-Key
        encrypted_bits = encrypt_decrypt_bits(payload_bits, self.private_key)
        
        print(f"[*] Text Payload: '{payload_text}'")
        print(f"[*] Bit length (including Null): {len(payload_bits)} bits")
        
        img = Image.open(image_path).convert("RGB")
        img_arr = np.array(img)
        height, width, _ = img_arr.shape
        
        # 2. Dynamic Block Sizing
        block_width = min(self.max_block_size, width)
        block_height = min(self.max_block_size, height)
        block_area = block_width * block_height
        
        primes = sieve_of_eratosthenes(block_area)
        
        if len(encrypted_bits) > len(primes):
            print(f"[!] Error: Image is too small (Area={block_area}px, Primes={len(primes)}). Not enough space.")
            return

        for y_start in range(0, height, block_height):
            for x_start in range(0, width, block_width):
                
                for bit_idx, bit_val in enumerate(encrypted_bits):
                    prime_pos = primes[bit_idx]
                    
                    local_x = prime_pos % block_width
                    local_y = prime_pos // block_width
                    
                    global_x = x_start + local_x
                    global_y = y_start + local_y
                    
                    if global_x < width and global_y < height:
                        green_val = img_arr[global_y, global_x, 1]
                        new_green = (green_val & 254) | bit_val
                        img_arr[global_y, global_x, 1] = new_green

        result_img = Image.fromarray(img_arr)
        result_img.save(output_path, "PNG")
        print(f"[+] Encrypted watermark embedded! Saved as: {output_path}")

    def extract(self, image_path):
        img = Image.open(image_path).convert("RGB")
        img_arr = np.array(img)
        height, width, _ = img_arr.shape
        
        # Dynamic Block Sizing
        block_width = min(self.max_block_size, width)
        block_height = min(self.max_block_size, height)
        block_area = block_width * block_height
        
        primes = sieve_of_eratosthenes(block_area)
        found_watermarks = set()
        
        for y_start in range(0, height, block_height):
            for x_start in range(0, width, block_width):
                
                extracted_bits = []
                for prime_pos in primes:
                    local_x = prime_pos % block_width
                    local_y = prime_pos // block_width
                    global_x = x_start + local_x
                    global_y = y_start + local_y
                    
                    if global_x < width and global_y < height:
                        green_val = img_arr[global_y, global_x, 1]
                        extracted_bits.append(green_val & 1)
                    else:
                        break 
                
                # Decrypt all extracted bits using XOR + PrivateKey
                decrypted_bits = encrypt_decrypt_bits(extracted_bits, self.private_key)
                
                try:
                    text = bits_to_text(decrypted_bits)
                    if text.startswith("coywin_"):
                        found_watermarks.add(text)
                except:
                    pass

        if found_watermarks:
            print("[+] Valid Coywin Watermark Found! Data:")
            for wm in found_watermarks:
                print(f"    -> {wm}")
            return list(found_watermarks)
        else:
            print("[-] Invalid image as Coywin output.")
            return []

if __name__ == "__main__":
    if len(sys.argv) > 1:
        mode = sys.argv[1]
        if mode == "embed" and len(sys.argv) == 5:
            # embed <input> <output> <private_key>
            cw = CoywinWatermark(sys.argv[4])
            cw.embed(sys.argv[2], sys.argv[3])
        elif mode == "extract" and len(sys.argv) == 4:
            # extract <input> <private_key>
            cw = CoywinWatermark(sys.argv[3])
            cw.extract(sys.argv[2])
        else:
            print("Usage: python coywin_watermark.py embed <input.png> <output.png> <private_key>")
            print("       python coywin_watermark.py extract <input.png> <private_key>")
