---
id: 4b2068dc-8b56-4d55-8ede-c900903ce3fa
parent: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
order: 5
tags:
- design-decision
- bug
- security
created: 2026-07-14T09:26:00Z
updated: 2026-07-14T09:26:00Z
---

# A keyring library silently lied about writing the credential

On macOS Sequoia, the `keyring` crate (v3)'s `set_password` returned
`Ok(())` without actually writing anything to the keychain — leaving a
`__keyring__` placeholder in `config.toml` with no matching entry
behind it, so the very next connection attempt failed with "No matching
entry found." The fix upgraded to keyring v4 and added a verification
step: `store_in_keyring` now does an immediate `get_password` right
after `set_password`, and only writes the `__keyring__` placeholder if
the secret is actually retrievable — otherwise it falls back to keeping
the plaintext URL rather than pointing at a credential that silently
doesn't exist.
