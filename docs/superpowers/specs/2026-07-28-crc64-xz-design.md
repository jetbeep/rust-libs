# CRC-64/XZ Support Design

## Scope

Add a `no_std` CRC-64/XZ facility to `jetbeep-core`. Its checksums must match
`crc64fast`, including `hello` and `123456789`.

## Dependency Layout

Declare `crc` 3.4.0 with default features disabled under
`[workspace.dependencies]`. `jetbeep-core` inherits that dependency, so all
workspace members use one compatible version when they need it.

## Public API

`jetbeep-core` exposes a `crc64` module with:

- `checksum(bytes: &[u8]) -> u64` for complete buffers.
- A re-export of the underlying digest type for incremental input.

Both paths use the `CRC_64_XZ` algorithm. This is compatible with the
`crc64fast` vectors while working on desktop and Zephyr targets.

## Validation

Unit tests cover the expected `hello` and `123456789` checksums and confirm
that updating a digest in chunks produces the same checksum as a one-shot
calculation.
