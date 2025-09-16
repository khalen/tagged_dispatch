use tagged_dispatch::tagged_dispatch;

#[tagged_dispatch]
trait Draw {
    fn draw(&self) -> &str;
    fn color(&self) -> &str {
        "default color"
    }
}

#[tagged_dispatch]
trait Geometry {
    fn area(&self) -> f32;
    fn perimeter(&self) -> f32;
}

#[tagged_dispatch]
trait Named {
    fn name(&self) -> &str;
}

#[tagged_dispatch(Draw, Geometry, Named)]
enum Shape {
    Circle(Circle),
    Rectangle(Rectangle),
    Triangle(Triangle),
}

#[derive(Clone)]
struct Circle {
    radius: f32,
}

impl Draw for Circle {
    fn draw(&self) -> &str {
        "Drawing circle"
    }

    fn color(&self) -> &str {
        "red"
    }
}

impl Geometry for Circle {
    fn area(&self) -> f32 {
        std::f32::consts::PI * self.radius * self.radius
    }

    fn perimeter(&self) -> f32 {
        2.0 * std::f32::consts::PI * self.radius
    }
}

impl Named for Circle {
    fn name(&self) -> &str {
        "Circle"
    }
}

#[derive(Clone)]
struct Rectangle {
    width: f32,
    height: f32,
}

impl Draw for Rectangle {
    fn draw(&self) -> &str {
        "Drawing rectangle"
    }
}

impl Geometry for Rectangle {
    fn area(&self) -> f32 {
        self.width * self.height
    }

    fn perimeter(&self) -> f32 {
        2.0 * (self.width + self.height)
    }
}

impl Named for Rectangle {
    fn name(&self) -> &str {
        "Rectangle"
    }
}

#[derive(Clone)]
struct Triangle {
    base: f32,
    height: f32,
    side1: f32,
    side2: f32,
    side3: f32,
}

impl Draw for Triangle {
    fn draw(&self) -> &str {
        "Drawing triangle"
    }

    fn color(&self) -> &str {
        "blue"
    }
}

impl Geometry for Triangle {
    fn area(&self) -> f32 {
        0.5 * self.base * self.height
    }

    fn perimeter(&self) -> f32 {
        self.side1 + self.side2 + self.side3
    }
}

impl Named for Triangle {
    fn name(&self) -> &str {
        "Triangle"
    }
}

#[test]
fn test_multiple_traits_dispatch() {
    let shapes: Vec<Shape> = vec![
        Shape::circle(Circle { radius: 2.0 }),
        Shape::rectangle(Rectangle { width: 3.0, height: 4.0 }),
        Shape::triangle(Triangle {
            base: 4.0,
            height: 3.0,
            side1: 3.0,
            side2: 4.0,
            side3: 5.0,
        }),
    ];

    for shape in &shapes {
        match shape.name() {
            "Circle" => {
                assert_eq!(shape.draw(), "Drawing circle");
                assert_eq!(shape.color(), "red");
                assert!((shape.area() - 12.566).abs() < 0.01);
                assert!((shape.perimeter() - 12.566).abs() < 0.01);
            }
            "Rectangle" => {
                assert_eq!(shape.draw(), "Drawing rectangle");
                assert_eq!(shape.color(), "default color");
                assert_eq!(shape.area(), 12.0);
                assert_eq!(shape.perimeter(), 14.0);
            }
            "Triangle" => {
                assert_eq!(shape.draw(), "Drawing triangle");
                assert_eq!(shape.color(), "blue");
                assert_eq!(shape.area(), 6.0);
                assert_eq!(shape.perimeter(), 12.0);
            }
            _ => panic!("Unknown shape"),
        }
    }
}

#[test]
fn test_enum_size_with_multiple_traits() {
    assert_eq!(std::mem::size_of::<Shape>(), 8);
}

#[cfg(feature = "allocator-bumpalo")]
#[test]
fn test_multiple_traits_with_arena() {
    // Create a separate enum with lifetime for arena tests
    #[tagged_dispatch(Draw, Geometry, Named)]
    enum ShapeArena<'a> {
        Circle(Circle),
        Rectangle(Rectangle),
        Triangle(Triangle),
    }

    let builder = ShapeArena::arena_builder();

    let circle = builder.circle(Circle { radius: 1.0 });
    let rect = builder.rectangle(Rectangle { width: 2.0, height: 3.0 });

    assert_eq!(circle.name(), "Circle");
    assert_eq!(circle.draw(), "Drawing circle");
    assert!((circle.area() - 3.1415).abs() < 0.01);

    assert_eq!(rect.name(), "Rectangle");
    assert_eq!(rect.draw(), "Drawing rectangle");
    assert_eq!(rect.area(), 6.0);

    let circle2 = circle;
    assert_eq!(circle.name(), circle2.name());
}