use tagged_dispatch::tagged_dispatch;

#[tagged_dispatch]
trait Draw {
    fn draw(&self) -> &str;
    fn area(&self) -> f32;
}

#[tagged_dispatch(Draw)]
enum Shape<'a> {
    Circle(Circle),
    Rectangle(Rectangle),
}

struct Circle {
    radius: f32,
}

impl Draw for Circle {
    fn draw(&self) -> &str {
        "Drawing circle"
    }

    fn area(&self) -> f32 {
        std::f32::consts::PI * self.radius * self.radius
    }
}

struct Rectangle {
    width: f32,
    height: f32,
}

impl Draw for Rectangle {
    fn draw(&self) -> &str {
        "Drawing rectangle"
    }

    fn area(&self) -> f32 {
        self.width * self.height
    }
}

#[cfg(feature = "allocator-bumpalo")]
#[test]
fn test_bumpalo_builder() {
    let builder = Shape::arena_builder();
    let circle = builder.circle(Circle { radius: 2.0 });
    let rect = builder.rectangle(Rectangle {
        width: 3.0,
        height: 4.0,
    });

    assert_eq!(circle.draw(), "Drawing circle");
    assert_eq!(rect.draw(), "Drawing rectangle");
    assert!((circle.area() - 12.566).abs() < 0.01);
    assert_eq!(rect.area(), 12.0);

    // Test that it's Copy
    let circle2 = circle;
    assert_eq!(circle.area(), circle2.area());
}

#[cfg(feature = "allocator-bumpalo")]
#[test]
fn test_bumpalo_reset() {
    use bumpalo::Bump;

    let mut builder = ShapeArenaBuilder::with_bumpalo();
    let initial_stats = builder.stats();

    let _circle = builder.circle(Circle { radius: 2.0 });
    let stats_after_alloc = builder.stats();
    assert!(stats_after_alloc.allocated_bytes > initial_stats.allocated_bytes);

    builder.reset();
    let stats_after_reset = builder.stats();
    // After reset, allocated bytes should be back to initial (or close)
    assert!(stats_after_reset.allocated_bytes <= initial_stats.allocated_bytes);
}

#[cfg(feature = "allocator-bumpalo")]
#[test]
fn test_external_bumpalo() {
    use bumpalo::Bump;

    let arena = Bump::new();
    let builder = ShapeArenaBuilder::with_external_bumpalo(&arena);

    let circle = builder.circle(Circle { radius: 1.0 });
    assert_eq!(circle.draw(), "Drawing circle");
}

#[cfg(feature = "allocator-typed-arena")]
#[test]
fn test_typed_arena_builder() {
    let builder = ShapeArenaBuilder::with_typed_arena();
    let circle = builder.circle(Circle { radius: 2.0 });
    let rect = builder.rectangle(Rectangle {
        width: 3.0,
        height: 4.0,
    });

    assert_eq!(circle.draw(), "Drawing circle");
    assert_eq!(rect.draw(), "Drawing rectangle");
}

#[cfg(all(feature = "allocator-bumpalo", feature = "allocator-typed-arena"))]
#[test]
fn test_both_allocators() {
    // Test that we can explicitly choose when both are available
    let builder_bump = ShapeArenaBuilder::with_bumpalo();
    let circle1 = builder_bump.circle(Circle { radius: 1.0 });

    let builder_typed = ShapeArenaBuilder::with_typed_arena();
    let circle2 = builder_typed.circle(Circle { radius: 1.0 });

    assert_eq!(circle1.draw(), circle2.draw());
    assert_eq!(circle1.area(), circle2.area());

    // Default should prefer bumpalo
    let builder_default = Shape::arena_builder();
    let circle3 = builder_default.circle(Circle { radius: 1.0 });
    assert_eq!(circle1.area(), circle3.area());
}

#[test]
fn test_size() {
    // Most importantly, the enum should be 8 bytes!
    assert_eq!(std::mem::size_of::<Shape>(), 8);
}