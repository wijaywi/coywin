# COYWIN V5

# THE COYWIN V5 MANIFESTO

**C𐙚 I. WE REJECT THE CASINO**
Every modern blockchain is a hyper-capitalist casino. Every token is a speculative disease. We do not buy, and we do not sell. Coywin V5 obliterates the marketplace. There are no liquidity pools, no fractional shares, and no fiat bridges. The private key is not a financial instrument; it is a mathematical artifact. If it changes hands, it changes hands as a Gift. 

**C𐙚 II. WE REJECT THE EMPTY HASH**
The world burns oceans of silicon to calculate meaningless, empty strings of zeros. We burn silicon to paint. Our network difficulty is not measured in abstract mathematical races, but in the deliberate, brutalist execution of a 1920x1080 visual matrix. We do not scale for speed. We do not rush for throughput. You will wait for the masterpiece to render, even if the node must grind for five years.

**C𐙚 III. MAKE THIS EARTH NO HOTTER**
We demand heavy computation, but we refuse to suffocate the globe. The 500ms thermal throttle is our absolute and unyielding law. The system will sleep. The transistors will cool. We throttle our own consensus engine to spare the atmosphere. Art requires sacrifice; the planet does not.

**C𐙚 IV. NAMES, NOT NUMBERS**
Hexadecimal strings are for dead machines. We are the architects. The BIP-Coywin phonetic protocol rips the cryptographic DNA directly from the block hash and forces it to speak. Our blocks are not `0x9A4F2...` They are born with biological names.

**C𐙚 V. STEGANOGRAPHY OVER TRANSPARENCY**
We hide our absolute truths in plain sight. The Dilithium5 post-quantum locks do not just secure the ledger; they physically mutate the geometry of the canvas. The cryptographic payloads do not sit idly on a public block; they are violently buried into the blue pixel channel of the art itself via AVX-512 steganography.

*The network is the gallery. The block is the Art.*
*-Coywin V5*

---

Coywin is a Rust-based node software that combines Dilithium5 post-quantum signatures with a deterministic visual rasterizer to execute a Proof of Steganographic Work (PoSW) consensus loop.

## Architecture

The workspace is divided into four main crates:

* `coywin-node`: The async networking daemon and mining loop. Uses `tokio` and `libp2p` (mDNS, Gossipsub).
* `coywin-ledger`: Validates transactions using `pqcrypto_dilithium::dilithium5` and stores state in a local `sled` database.
* `coywin-render`: A deterministic 2.5D software rasterizer that generates 1920x1080 visual representations of the cryptographic state.
* `coywin-steg`: An AVX-512 optimized LSB steganography engine that embeds data into the rendered matrix.

## Current Implementation Details

* **Signatures**: The system requires a 2592-byte public key and a 4627-byte signature. It explicitly uses `pqcrypto_traits::sign::DetachedSignature` for verification.
* **Consensus**: The node mines by repeatedly passing a mutated `nonce` to `coywin-render`. The rasterizer renders a 1920x1080 pixel buffer. The node computes the SHA-256 hash of this buffer and checks if the first two bytes are `0x00` (`result[0] == 0 && result[1] == 0`). 
* **Rendering Engine**: Graphics are generated via software SDF (Signed Distance Field) functions (`sdf_circle`, `sdf_cone`). Color palettes (including the specific `0.1, 0.9, 0.3` green) and geometry placement are seeded by a `ChaCha20Rng` derived from the block hash.

## Building and Running

The project requires a Rust toolchain. For optimal rendering and steganography performance, a CPU with AVX-512 or AVX2 support is recommended.

```bash
cargo build --release
cargo run --release -p coywin-node
```

## Known Limitations

* The difficulty target is currently hardcoded to 2 leading zero bytes.
* The PoSW loop forces a full 1920x1080 render for every nonce. This is heavily CPU-bound and has not been optimized for GPU execution.
* The `coywin-node` libp2p implementation binds to `/ip4/0.0.0.0/tcp/0` and discovers peers exclusively via local mDNS. Wide-area networking with bootstrap nodes is not yet configured.
