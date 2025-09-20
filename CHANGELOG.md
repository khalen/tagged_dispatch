# Changelog

## [0.3.0] - 2025-01-20

### Added
- Automatic trait implementations for generated enums: `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`
- Trait implementations for base `TaggedPtr<T>` type
- Control flags to opt out of automatic trait generation:
  - `no_debug` - Skip Debug implementation
  - `no_eq` - Skip PartialEq/Eq implementations
  - `no_ord` - Skip PartialOrd/Ord implementations
  - `no_cmp` - Skip all comparison traits
  - `no_traits` - Skip all automatic trait implementations

### Changed
- **BREAKING**: Generated enums now automatically implement common traits. If you have existing implementations of `Debug`, `PartialEq`, `Eq`, `PartialOrd`, or `Ord` for your tagged dispatch enums, you'll need to either:
  - Remove your implementations and use the automatically generated ones
  - Add the appropriate opt-out flag (e.g., `no_debug`) to keep your custom implementation

### Implementation Notes
- All comparison traits use pointer equality - two instances are equal only if they point to the same object
- Debug implementation shows the enum and variant names (e.g., `"Shape::Circle"`)
- Ordering compares by variant type first (tag), then by pointer address

## [0.2.1] - 2024-12-XX

### Added
- Re-export allocator crates to simplify dependencies
- Improved documentation

## [0.2.0] - 2024-12-XX

### Added
- Arena allocation support with `bumpalo` and `typed-arena`
- Support for multiple trait dispatch
- Builder pattern for arena-allocated instances

### Changed
- Simplified variant syntax (can write `Circle` instead of `Circle(Circle)`)

## [0.1.0] - 2024-12-XX

### Initial Release
- Basic tagged dispatch functionality
- 8-byte enum size regardless of variant types
- Zero-cost dispatch without vtables
- Apple Silicon TBI optimization