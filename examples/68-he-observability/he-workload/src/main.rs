//! he-workload — a tiny homomorphic-encryption workload built on TFHE-rs.
//!
//! Each homomorphic operation is wrapped in a `#[no_mangle] #[inline(never)]
//! pub extern "C"` boundary function so the observer has a stable symbol to
//! uprobe — exactly the technique from Chapter 14. An optimized release build
//! would otherwise inline and monomorphize the library calls away. State lives
//! in a `static` so the boundary signatures stay trivial to probe.
//!
//! The operands are ciphertext throughout; the only place plaintext exists is
//! the final local decrypt inside `he_decrypt`, which the observer never reads.
use std::sync::Mutex;
use std::time::Duration;

use tfhe::prelude::*;
use tfhe::{generate_keys, set_server_key, ClientKey, ConfigBuilder, FheUint8};

struct HeState {
    client_key: ClientKey,
    a: FheUint8,
    b: FheUint8,
    c: FheUint8,
}

static STATE: Mutex<Option<HeState>> = Mutex::new(None);

/// Generate client/server keys and seed two ciphertexts. Expensive — this is
/// the cost FHE pays up front, and (with bootstrapping inside `compute`) the
/// reason the workload is orders of magnitude slower than plaintext.
#[no_mangle]
#[inline(never)]
pub extern "C" fn he_keygen() {
    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);
    set_server_key(server_key);
    let a = FheUint8::encrypt(7u8, &client_key);
    let b = FheUint8::encrypt(5u8, &client_key);
    let c = a.clone();
    *STATE.lock().unwrap() = Some(HeState { client_key, a, b, c });
}

/// Re-encrypt the two operands. Produces fresh ciphertext from plaintext.
#[no_mangle]
#[inline(never)]
pub extern "C" fn he_encrypt() {
    let mut guard = STATE.lock().unwrap();
    let s = guard.as_mut().expect("keygen first");
    s.a = FheUint8::encrypt(7u8, &s.client_key);
    s.b = FheUint8::encrypt(5u8, &s.client_key);
}

/// The homomorphic operation: multiply two ciphertexts WITHOUT decrypting.
/// This is where the time goes (programmable bootstrapping, NTT-heavy
/// polynomial multiplication).
#[no_mangle]
#[inline(never)]
pub extern "C" fn he_compute() {
    let mut guard = STATE.lock().unwrap();
    let s = guard.as_mut().expect("keygen first");
    s.c = &s.a * &s.b;
}

/// Decrypt the result locally. The only plaintext in the program — and the
/// observer cannot see it: it times this function, it does not read its return.
#[no_mangle]
#[inline(never)]
pub extern "C" fn he_decrypt() -> u8 {
    let guard = STATE.lock().unwrap();
    let s = guard.as_ref().expect("keygen first");
    s.c.decrypt(&s.client_key)
}

fn main() {
    println!("he-workload pid {} — keygen, then encrypt/compute/decrypt loop", std::process::id());
    he_keygen();
    let mut i: u64 = 0;
    loop {
        he_encrypt();
        he_compute();
        let r = he_decrypt();
        // 7 * 5 = 35; printed locally only, never crosses to the observer.
        println!("iter {i}: decrypted result = {r}");
        // Occasionally rotate keys so the observer also catches keygen latency.
        if i % 8 == 7 {
            he_keygen();
        }
        i += 1;
        std::thread::sleep(Duration::from_millis(500));
    }
}
