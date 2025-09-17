use std::fs;

use ring::{
    rand,
    signature::{Ed25519KeyPair, KeyPair},
};

fn main() -> std::io::Result<()> {
    let rng = rand::SystemRandom::new();

    let pkcs8_doc = Ed25519KeyPair::generate_pkcs8(&rng).expect("unable to generate key");
    let pkcs8_key =
        Ed25519KeyPair::from_pkcs8(pkcs8_doc.as_ref()).expect("unable to parse generated key");

    fs::write("privkey.der", pkcs8_doc.as_ref())?;
    fs::write("pubkey.der", pkcs8_key.public_key().as_ref())?;

    println!("Key created!");

    Ok(())
}
