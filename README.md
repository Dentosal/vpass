# vpass - An opinionated password manager

Password manager library / backend / CLI.


## Security

This program has not been audited, and might not be secure. However, I'm not aware of any vulnerabilities or weaknesses.

All cryptography is done using [libsodium](https://github.com/jedisct1/libsodium) through [rust_sodium](https://github.com/maidsafe/rust_sodium).
Passwords vaults are encrypted using `Salsa20` and authenticity is validated `Poly1305`,
as described in [`rust_sodium` documentation](https://docs.rs/rust_sodium/0.10.2/rust_sodium/crypto/secretbox/index.html).
