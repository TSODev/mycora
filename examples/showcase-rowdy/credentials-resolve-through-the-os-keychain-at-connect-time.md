---
id: 45c3abec-4c75-4656-b027-477f6eed663c
parent: 6d5eb76b-9b62-4c58-96e3-90c1e30f7fc7
order: 4
tags:
- architecture
- security
created: 2026-07-14T09:09:00Z
updated: 2026-07-14T09:09:00Z
---

# Credentials resolve through the OS keychain at connect time

`src/config.rs` resolves `__keyring__` placeholders against the OS
keychain (the `keyring` crate, feature `secure-storage`, on by default)
only at the moment a connection is made — a saved profile's
`config.toml` entry never holds a plaintext secret once encryption is
set up. The same file's `redact_url`/`strip_readonly_param` mask
secrets in anything shown on screen or logged, and parse
`?readonly=true` off the connection URL. See [[A keyring library silently lied about writing the credential]] for a real bug this design
surfaced.
