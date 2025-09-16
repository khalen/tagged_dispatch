# tagged_dispatch

Memory-efficient trait dispatch using tagged pointers. Like `enum_dispatch`, but your enums are only 8 bytes regardless of the variant size!

## Features

- **8-byte enums** - Constant size regardless of variant types
- **Zero-cost dispatch** - Inlined, no vtable overhead
- **Familiar API** - Works like `enum_dispatch`
- **No allocator required** - Works with `no_std` (bring your own allocator)
- **Cache-friendly** - Better locality than fat enums

## Example
```rust
use tagged_dispatch::tagged_dispatch;

#[tagged_dispatch]
trait Draw {
    fn draw(&self);
    fn area(&self) -> f32;
}

#[tagged_dispatch(Draw)]
enum Shape {
    Circle(Circle),
    Rectangle(Rectangle),
    Triangle(Triangle),
}

struct Circle { radius: f32 }
impl Draw for Circle {
    fn draw(&self) { /* ... */ }
    fn area(&self) -> f32 { 
        std::f32::consts::PI * self.radius * self.radius 
    }
}

// ... implement for Rectangle and Triangle ...

fn main() {
    let shape = Shape::circle(Circle { radius: 5.0 });
    shape.draw();
    
    // Only 8 bytes, not size_of::<largest variant>()!
    assert_eq!(std::mem::size_of::<Shape>(), 8);
}

