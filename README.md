# Coywin V5

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
