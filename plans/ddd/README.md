# Verdant Domain-Driven Design

## Documents

| Document | Purpose |
|----------|---------|
| [Domain Model](domain-model.md) | Aggregates, entities, value objects with London School test examples for each aggregate |
| [Bounded Contexts](bounded-contexts.md) | Context map, context boundaries, ports/adapters, anti-corruption layers with mock-based tests at every boundary |
| [Domain Events](domain-events.md) | Complete event catalog, event flow diagrams, event handler testing patterns |
| [Testing Strategy](testing-strategy.md) | Full London School TDD methodology — test pyramid, outside-in walkthrough, mocking guidelines, CI pipeline |

## Key Principles

1. **Trait boundaries everywhere** — every bounded context communicates through Rust traits (ports). This enables London School mocking on x86 without hardware.

2. **Outside-in development** — start from acceptance tests in `verdant-sim`, discover interfaces working inward to firmware.

3. **Three test layers** — London School mocks (fast, many), property tests (algorithmic), simulation tests (end-to-end).

4. **Data sovereignty as shared kernel** — `Local<T>`, `ZoneEncrypted<T>`, `MeshPublic<T>` enforce privacy at compile time across all contexts.
