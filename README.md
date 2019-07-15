# vpass - An opinionated password manager

Password manager library / backend / CLI.

## Features (completed / planned)

- [x] Stores full password history
    - [ ] CLI access to history
- [x] Small easy-to-read codebase
- [x] Machine-readable command line output
- [x] Atomic file updates
    - Local changes are always atomic, and synchronization is applied in a separate pass
- [x] Synchronization through multiple providers
    - [x] GitHub repositories (through API)
    - [x] Other filesystem locations
    - [ ] SSH filesystem
    - [ ] Git
    - [ ] S3 Buckets
- [ ] Web interface
- [ ] Web browser plugins
- [ ] Batch imports from other password managers
- [ ] System keychain integration
- [ ] Shared vaults

## Concepts

### Vault

An encrypted book.

### Book

A container for password entries.

## Security

This program has not been audited, and might not be secure. However, I'm not aware of any vulnerabilities or weaknesses.

All cryptography is done using [libsodium](https://github.com/jedisct1/libsodium) through [rust_sodium](https://github.com/maidsafe/rust_sodium).
Passwords vaults are encrypted using `Salsa20` and authenticity is validated `Poly1305`,
as described in [`rust_sodium` documentation](https://docs.rs/rust_sodium/0.10.2/rust_sodium/crypto/secretbox/index.html).
