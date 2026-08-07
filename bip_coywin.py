import os

KAMUS_PATH = r"D:\zzzzzzzzzzz AntiGravity\zzz TEMP\Jenis kAta\Kamus_Suku_Kata_12bit_4096_R_Winjay_Final.csv"

def load_dictionary():
    words = []
    try:
        with open(KAMUS_PATH, 'r', encoding='utf-8') as f:
            for line in f:
                word = line.strip()
                if word:
                    words.append(word)
    except Exception as e:
        print(f"[ERROR] Failed to load BIP-Coywin dictionary: {e}")
    return words

KAMUS = load_dictionary()

def hash_to_name(hex_hash):
    """
    Takes a 64-char hex hash.
    Reverses it to avoid PoW leading zeros.
    Takes 12 hex chars (4 chunks of 3 chars).
    Each 3 hex chars (12-bit) is mapped to an index (0-4095) in the dictionary.
    Returns a 4-syllable capitalized name.
    """
    if not KAMUS:
        return "Unknown"
        
    # Ambil 12 karakter dari TENGAN hash (index 26 sampai 38)
    # Ini memastikan kita selalu mendapatkan porsi yang paling acak, 
    # terlepas dari apakah hash tersebut sudah di-reverse (dibalik) sebelumnya atau belum.
    middle_chunk = hex_hash[26:38]
    
    name_parts = []
    for i in range(4):
        chunk = middle_chunk[i*3 : (i+1)*3]
        idx = int(chunk, 16) % len(KAMUS)
        name_parts.append(KAMUS[idx])
        
    full_name = "".join(name_parts)
    return full_name.capitalize()
