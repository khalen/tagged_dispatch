# tagged_dispatch

[![Crates.io](https://img.shields.io/crates/v/tagged_dispatch.svg)](https://crates.io/crates/tagged_dispatch)
[![Documentation](https://docs.rs/tagged_dispatch/badge.svg)](https://docs.rs/tagged_dispatch)
[![License](https://img.shields.io/crates/l/tagged_dispatch.svg)](https://github.com/khalen/tagged_dispatch#license)

Memory-efficient trait dispatch using tagged pointers. Like `enum_dispatch`, but uses only 8 bytes per instance with heap-allocated variants instead of stack-allocated ones the size of the largest variant.

## Features

- **8-byte enums** - Constant size regardless of variant types
- **Zero-cost dispatch** - Inlined, no vtable overhead
- **No allocator required** - Works with `no_std` (bring your own allocator)
- **Cache-friendly** - Better locality than fat enums
- **Arena allocation support** - Optional arena allocation for even better performance
- **Apple Silicon optimized** - Leverages ARM64 TBI for zero-cost tag removal

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
tagged_dispatch = "0.1"

# Optional: Enable arena allocation support
tagged_dispatch = { version = "0.1", features = ["allocator-bumpalo"] }
```

### Feature Flags

- `std` (default): Standard library support
- `allocator-bumpalo`: Implements `TaggedAllocator` for `bumpalo::Bump`
- `allocator-typed-arena`: Implements `TaggedAllocator` for `typed_arena::Arena<T>`
- `all-allocators`: Enables all allocator implementations

## Quick Example

```rust
use tagged_dispatch::tagged_dispatch;

// Define your trait
#[tagged_dispatch]
trait Draw {
    fn draw(&self);
    fn area(&self) -> f32;
}

// Create an enum with variants that implement the trait
#[tagged_dispatch(Draw)]
enum Shape {
    Circle,      // Expands to Circle(Circle)
    Rectangle,
    Triangle,
}

// Implement the trait for each variant
#[derive(Clone)]
struct Circle { radius: f32 }

impl Draw for Circle {
    fn draw(&self) {
        println!("Drawing a circle with radius {}", self.radius);
    }

    fn area(&self) -> f32 {
        std::f32::consts::PI * self.radius * self.radius
    }
}

#[derive(Clone)]
struct Rectangle { width: f32, height: f32 }

impl Draw for Rectangle {
    fn draw(&self) {
        println!("Drawing a {}x{} rectangle", self.width, self.height);
    }

    fn area(&self) -> f32 {
        self.width * self.height
    }
}

#[derive(Clone)]
struct Triangle { base: f32, height: f32 }

impl Draw for Triangle {
    fn draw(&self) {
        println!("Drawing a triangle with base {} and height {}", self.base, self.height);
    }

    fn area(&self) -> f32 {
        0.5 * self.base * self.height
    }
}

// Create shapes using generated constructors
let shapes = vec![
    Shape::circle(Circle { radius: 5.0 }),
    Shape::rectangle(Rectangle { width: 10.0, height: 5.0 }),
    Shape::triangle(Triangle { base: 8.0, height: 6.0 }),
];

// Dispatch trait methods
for shape in &shapes {
    shape.draw();
    println!("Area: {}", shape.area());
}

// Only 8 bytes per enum, not size_of::<largest variant>()!
assert_eq!(std::mem::size_of::<Shape>(), 8);
```

## When to Use

### Use `tagged_dispatch` when:
- You have many instances and memory usage is critical (8 bytes vs potentially hundreds)
- Your variants are large or vary significantly in size
- You can accept the heap allocation overhead
- You want better cache locality for collections

### Use `enum_dispatch` when:
- You want stack allocation and no heap overhead
- Your variants are similarly sized or small
- You have fewer instances
- You need the absolute fastest dispatch (no pointer indirection)

### Use trait objects when:
- You need open sets of types (not known at compile time)
- You're okay with 16-byte fat pointers
- You need to work with external types you don't control

## Memory Models

### Owned Mode (Default)

Without lifetime parameters on the enum, generates owned tagged pointers using `Box`:
- Variants are allocated with `Box::into_raw(Box::new(value))`
- Implements `Drop` to deallocate
- Has non-trivial `Clone` that deep-copies

### Arena Mode

With lifetime parameters on the enum, generates arena-allocated pointers:
- Variants allocated through `TaggedAllocator` trait
- Types are `Copy` (just copies the 8-byte pointer)
- Arena manages object lifetimes
- Variants don't need to be `Send`, `Sync`, or even `Sized`

## Advanced Features

### Arena Allocation

For high-performance scenarios, use arena allocation to get `Copy` types and eliminate individual allocations:

```rust
#[cfg(feature = "allocator-bumpalo")]
{
    use tagged_dispatch::tagged_dispatch;

    #[tagged_dispatch]
    trait Process {
        fn process(&self, value: i32) -> i32;
    }

    #[tagged_dispatch(Process)]
    enum Processor<'a> {  // Note the lifetime parameter
        Doubler,
        Squarer,
    }

    #[derive(Clone)]
    struct Doubler;
    impl Process for Doubler {
        fn process(&self, value: i32) -> i32 { value * 2 }
    }

    #[derive(Clone)]
    struct Squarer;
    impl Process for Squarer {
        fn process(&self, value: i32) -> i32 { value * value }
    }

    // Create an arena builder
    let builder = Processor::arena_builder();

    // Allocate variants in the arena
    let proc1 = builder.doubler(Doubler);
    let proc2 = builder.squarer(Squarer);

    // These are Copy and 8 bytes each.
    let proc3 = proc1;

    assert_eq!(proc1.process(5), 10);
    assert_eq!(proc2.process(5), 25);
    assert_eq!(proc3.process(5), 10);
}
```

### Multiple Trait Dispatch

Dispatch multiple traits through the same enum:

```rust
use tagged_dispatch::tagged_dispatch;

#[tagged_dispatch]
trait Draw {
    fn draw(&self);
}

#[tagged_dispatch]
trait Serialize {
    fn serialize(&self) -> String;
}

#[tagged_dispatch(Draw, Serialize)]
enum Shape {
    Circle,      // Simplified syntax
    Rectangle,
}

// Complete the example with struct definitions
#[derive(Clone)]
struct Circle { radius: f32 }

impl Draw for Circle {
    fn draw(&self) {
        println!("Drawing circle");
    }
}

impl Serialize for Circle {
    fn serialize(&self) -> String {
        format!("Circle({})", self.radius)
    }
}

#[derive(Clone)]
struct Rectangle { width: f32, height: f32 }

impl Draw for Rectangle {
    fn draw(&self) {
        println!("Drawing rectangle");
    }
}

impl Serialize for Rectangle {
    fn serialize(&self) -> String {
        format!("Rectangle({}x{})", self.width, self.height)
    }
}

// Example usage
let shape = Shape::circle(Circle { radius: 5.0 });
shape.draw();
assert_eq!(shape.serialize(), "Circle(5)");
```

### Default Implementations

Traits with default implementations work as expected:

```rust
use tagged_dispatch::tagged_dispatch;

#[tagged_dispatch]
trait Animal {
    fn make_sound(&self) -> &str;

    fn legs(&self) -> u32 {
        4  // Default implementation
    }
}

#[tagged_dispatch(Animal)]
enum Pet {
    Dog,
    Bird,
}

#[derive(Clone)]
struct Dog;

impl Animal for Dog {
    fn make_sound(&self) -> &str {
        "Woof!"
    }
    // Uses default legs() implementation (4)
}

#[derive(Clone)]
struct Bird;

impl Animal for Bird {
    fn make_sound(&self) -> &str {
        "Tweet!"
    }

    fn legs(&self) -> u32 {
        2  // Override default
    }
}

// Example usage
let dog = Pet::dog(Dog);
assert_eq!(dog.make_sound(), "Woof!");
assert_eq!(dog.legs(), 4);  // Uses default

let bird = Pet::bird(Bird);
assert_eq!(bird.make_sound(), "Tweet!");
assert_eq!(bird.legs(), 2);  // Overridden
```

### Non-Dispatched Methods

Mark trait methods that shouldn't be dispatched with `#[no_dispatch]`:

```rust
use tagged_dispatch::tagged_dispatch;

#[tagged_dispatch]
trait MyTrait {
    fn dispatched(&self) -> i32;

    #[no_dispatch]
    fn not_dispatched() -> &'static str {
        "This won't be dispatched"
    }
}

#[tagged_dispatch(MyTrait)]
enum Value {
    First,
    Second,
}

#[derive(Clone)]
struct First(i32);

impl MyTrait for First {
    fn dispatched(&self) -> i32 {
        self.0
    }
}

#[derive(Clone)]
struct Second(i32);

impl MyTrait for Second {
    fn dispatched(&self) -> i32 {
        self.0 * 2
    }
}

// Example usage
let val = Value::first(First(5));
assert_eq!(val.dispatched(), 5);  // This is dispatched

// Static method is called on the concrete type, not the enum
assert_eq!(<First as MyTrait>::not_dispatched(), "This won't be dispatched");
```

## Architecture Requirements

This crate requires x86-64 or AArch64 architectures where the top 7 bits of 64-bit pointers are unused (standard on modern Linux, macOS, and Windows systems).

### Platform Optimizations

**Apple Silicon (macOS ARM64)**: This crate automatically leverages the ARM64 Top Byte Ignore (TBI) feature on Apple Silicon Macs. TBI allows the processor to automatically ignore the top byte of pointers during memory access, eliminating the need for software masking. This provides a measurable performance improvement by removing a bitwise AND operation from every pointer dereference in the dispatch path.

## Limitations

- Supports up to 128 variant types (7-bit tag)
- Generic traits are not supported
- Requires heap allocation for variants (or arena allocation)
- Only works on x86-64 and AArch64 architectures

## Safety

This crate uses `unsafe` code for tagged pointer manipulation. I've tried to carefully document and test all unsafe operations.

### Safety Invariants

1. **Valid Pointers**: All pointers stored in `TaggedPtr` are valid, properly aligned, and point to initialized data
2. **Tag Range**: Tags are always within the valid range (0-127), enforced by debug assertions
3. **Memory Management**: Proper cleanup via `Drop` implementation (in the default boxed implementation) ensures no memory leaks
4. **Type Safety**: Type safety is enforced at compile time through the macro-generated code

### Unsafe Operations

The crate contains the following unsafe operations:

1. **Pointer Dereferencing** (`TaggedPtr::as_ref`, `TaggedPtr::as_mut`):
   - Safety: Caller must ensure the pointer is valid and properly initialized
   - Used by generated dispatch code to access variant data

2. **Memory Deallocation** (in generated `Drop` impl):
   - Safety: Uses `untagged_ptr()` to ensure the original pointer is passed to `Box::from_raw`
   - Prevents memory leaks by properly deallocating heap-allocated variants

3. **Type Transmutation** (in generated code):
   - Safety: Tag values are guaranteed to map to valid enum discriminants
   - Used to convert between tag values and enum variant types

4. **Send/Sync Implementation**:
   - Safety: `TaggedPtr<T>` is `Send`/`Sync` if and only if `T` is `Send`/`Sync`
   - Preserves thread safety guarantees of the underlying types

All unsafe code is contained within the library implementation and is not exposed to users.

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/khalen/tagged_dispatch/blob/master/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license ([LICENSE-MIT](https://github.com/khalen/tagged_dispatch/blob/master/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
