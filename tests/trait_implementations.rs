use tagged_dispatch::tagged_dispatch;

#[tagged_dispatch]
trait Draw {
    fn draw(&self) -> &str;
}

#[derive(Clone)]
struct Circle {
    radius: f32,
}

impl Draw for Circle {
    fn draw(&self) -> &str {
        "circle"
    }
}

#[derive(Clone)]
struct Rectangle {
    width: f32,
    height: f32,
}

impl Draw for Rectangle {
    fn draw(&self) -> &str {
        "rectangle"
    }
}

#[tagged_dispatch(Draw)]
enum Shape {
    Circle,
    Rectangle,
}

#[test]
fn test_debug_implementation() {
    let circle = Shape::circle(Circle { radius: 1.0 });
    let rect = Shape::rectangle(Rectangle { width: 2.0, height: 3.0 });

    let circle_debug = format!("{:?}", circle);
    let rect_debug = format!("{:?}", rect);

    assert!(circle_debug.contains("Shape::Circle"));
    assert!(rect_debug.contains("Shape::Rectangle"));
}

#[test]
fn test_equality() {
    let circle1 = Shape::circle(Circle { radius: 1.0 });
    let circle2 = Shape::circle(Circle { radius: 1.0 });
    let circle3 = circle1.clone();

    // Different objects, even with same data, should not be equal (pointer equality)
    assert_ne!(circle1, circle2);

    // Same object should be equal to itself
    assert_eq!(circle1, circle1);

    // Cloned objects are different (new allocation)
    assert_ne!(circle1, circle3);
}

#[test]
fn test_ordering() {
    let mut shapes = vec![
        Shape::rectangle(Rectangle { width: 1.0, height: 2.0 }),
        Shape::circle(Circle { radius: 1.0 }),
        Shape::rectangle(Rectangle { width: 3.0, height: 4.0 }),
    ];

    // Should be able to sort without panicking
    shapes.sort();

    // After sorting, all Circles should come before all Rectangles
    // (since Circle tag = 0, Rectangle tag = 1)
    let tags: Vec<_> = shapes.iter().map(|s| s.tag_type()).collect();

    // Check that tags are in order
    for i in 1..tags.len() {
        assert!(tags[i-1] <= tags[i]);
    }
}

#[test]
fn test_derives_work() {
    // Test that we can use derives on types containing our tagged pointer types
    #[derive(Debug, PartialEq, Eq)]
    struct Container {
        shape: Shape,
        name: String,
    }

    let container1 = Container {
        shape: Shape::circle(Circle { radius: 1.0 }),
        name: "test".to_string(),
    };

    let container2 = Container {
        shape: Shape::circle(Circle { radius: 1.0 }),
        name: "test".to_string(),
    };

    // Different Shape pointers, so containers are not equal
    assert_ne!(container1, container2);

    // Can format debug
    let debug_str = format!("{:?}", container1);
    assert!(debug_str.contains("Container"));
    assert!(debug_str.contains("Shape::Circle"));
}

#[cfg(feature = "allocator-bumpalo")]
#[test]
fn test_arena_version_traits() {
    #[tagged_dispatch(Draw)]
    enum ShapeArena<'a> {
        Circle,
        Rectangle,
    }

    let builder = ShapeArena::arena_builder();

    let circle1 = builder.circle(Circle { radius: 1.0 });
    let circle2 = builder.circle(Circle { radius: 1.0 });

    // Different allocations should have different pointers
    assert_ne!(circle1, circle2);

    // Copy should be equal (same pointer)
    let circle3 = circle1;
    assert_eq!(circle1, circle3);

    // Debug should work
    let debug = format!("{:?}", circle1);
    assert!(debug.contains("ShapeArena::Circle"));

    // Should be orderable
    assert!(circle1 < circle2 || circle1 > circle2);
}