use tagged_dispatch::tagged_dispatch;
use std::fmt;

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

// Test opting out of Debug
#[tagged_dispatch(Draw, no_debug)]
enum ShapeNoDebug {
    Circle,
    Rectangle,
}

// Custom Debug implementation
impl fmt::Debug for ShapeNoDebug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.tag_type() {
            ShapeNoDebugType::Circle => write!(f, "🟢 Circle shape"),
            ShapeNoDebugType::Rectangle => write!(f, "⬜ Rectangle shape"),
        }
    }
}

#[test]
fn test_custom_debug() {
    let circle = ShapeNoDebug::circle(Circle { radius: 1.0 });
    let rect = ShapeNoDebug::rectangle(Rectangle { width: 2.0, height: 3.0 });

    let circle_debug = format!("{:?}", circle);
    let rect_debug = format!("{:?}", rect);

    // Our custom implementation
    assert!(circle_debug.contains("🟢"));
    assert!(rect_debug.contains("⬜"));

    // Should still have equality from base TaggedPtr
    assert_eq!(circle, circle);
    assert_ne!(circle, rect);
}

// Test opting out of comparison traits
#[tagged_dispatch(Draw, no_cmp)]
enum ShapeNoCompare {
    Circle,
    Rectangle,
}

#[test]
fn test_no_compare() {
    let circle1 = ShapeNoCompare::circle(Circle { radius: 1.0 });
    let circle2 = ShapeNoCompare::circle(Circle { radius: 1.0 });

    // Should still have Debug
    let debug = format!("{:?}", circle1);
    assert!(debug.contains("ShapeNoCompare"));

    // The following should not compile if we correctly omitted PartialEq
    // Uncomment to verify compilation fails:
    // assert_eq!(circle1, circle2);
}

// Test opting out of Ord but keeping Eq
#[tagged_dispatch(Draw, no_ord)]
enum ShapeNoOrd {
    Circle,
    Rectangle,
}

#[test]
fn test_no_ord() {
    let circle = ShapeNoOrd::circle(Circle { radius: 1.0 });
    let rect = ShapeNoOrd::rectangle(Rectangle { width: 2.0, height: 3.0 });

    // Should have equality
    assert_eq!(circle, circle);
    assert_ne!(circle, rect);

    // The following should not compile if we correctly omitted Ord
    // Uncomment to verify compilation fails:
    // let mut shapes = vec![circle, rect];
    // shapes.sort();
}

// Test opting out of all traits
#[tagged_dispatch(Draw, no_traits)]
enum ShapeNoTraits {
    Circle,
    Rectangle,
}

// Must implement all traits manually
impl fmt::Debug for ShapeNoTraits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ShapeNoTraits")
    }
}

impl PartialEq for ShapeNoTraits {
    fn eq(&self, other: &Self) -> bool {
        // Custom logic - only compare tags
        self.tag_type() == other.tag_type()
    }
}

impl Eq for ShapeNoTraits {}

#[test]
fn test_no_traits() {
    let circle1 = ShapeNoTraits::circle(Circle { radius: 1.0 });
    let circle2 = ShapeNoTraits::circle(Circle { radius: 2.0 });
    let rect = ShapeNoTraits::rectangle(Rectangle { width: 1.0, height: 1.0 });

    // Our custom Debug
    assert_eq!(format!("{:?}", circle1), "ShapeNoTraits");

    // Our custom equality (only compares tags, not pointers)
    assert_eq!(circle1, circle2); // Different objects but same variant type
    assert_ne!(circle1, rect);
}

// Test arena version with flags
#[cfg(feature = "allocator-bumpalo")]
mod arena_tests {
    use super::*;

    #[tagged_dispatch(Draw, no_debug)]
    enum ShapeArena<'a> {
        Circle,
        Rectangle,
    }

    impl<'a> fmt::Debug for ShapeArena<'a> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Arena shape: {:?}", self.tag_type())
        }
    }

    #[test]
    fn test_arena_custom_debug() {
        let builder = ShapeArena::arena_builder();
        let circle = builder.circle(Circle { radius: 1.0 });

        let debug = format!("{:?}", circle);
        assert!(debug.contains("Arena shape"));

        // Should still have equality
        let circle2 = circle;
        assert_eq!(circle, circle2);
    }
}