# Ferivonus Macros

Internal procedural macro engine for the Ferivonus documentation system.

This crate provides the attribute macros used to extract metadata from Actix-web route handlers at compile time. It is designed to work in tandem with the `ferivonus-swagger-gen` crate.

## Purpose

The primary role of this crate is to implement the `#[register_api]` macro. This macro:

1. Parses function attributes to identify HTTP methods (GET, POST, etc.) and paths.
2. Extracts custom documentation metadata provided by the user (summaries, parameters, types).
3. Injects code to submit this data to a global registry using the `inventory` crate.

## Technical Overview

The macro performs a static analysis of the function signature:

- **Path Extraction:** It looks into `#[get("/path")]` or similar Actix attributes. If not found, it defaults to the function name.
- **Method Detection:** It identifies the HTTP verb from the Actix attribute.
- **Metadata Injection:** It wraps the function and adds a static initialization block that runs before the server starts.

## Usage

This crate is a dependency of `ferivonus-swagger-gen`. To use the documentation engine in your project, please refer to the main crate:

```toml
[dependencies]
ferivonus-swagger-gen = "0.1.4"
```
