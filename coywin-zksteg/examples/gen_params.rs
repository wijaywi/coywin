
use coywin_zksteg::generate_params;
use std::fs::File;
use std::io::Write;

fn main() {
    let k = 8;
    let params = generate_params(k);
    let mut buf = vec![];
    params.write(&mut buf).expect("serialize params");
    let mut f = File::create("coywin-client/assets/coywin_ipa_params.bin").unwrap_or_else(|_| File::create("../coywin-client/assets/coywin_ipa_params.bin").unwrap());
    f.write_all(&buf).unwrap();
    println!("Wrote {} bytes, k={}", buf.len(), k);
}

