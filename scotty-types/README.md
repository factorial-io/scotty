# scotty-types

Minimal types-only crate for TypeScript generation.

This crate contains shared type definitions used across the Scotty project. Types are automatically exported to TypeScript using [ts-rs](https://github.com/Aleph-Alpha/ts-rs).

## Features

- `utoipa`: Optional support for OpenAPI documentation generation

## Dependencies

- `serde`: Serialization/deserialization
- `chrono`: Date and time types
- `uuid`: UUID support
- `ts-rs`: TypeScript binding generation
