use tagged_dispatch::tagged_dispatch;

#[tagged_dispatch]
trait Operations {
    fn compute(&self) -> i32;

    #[no_dispatch]
    fn static_info() -> &'static str {
        "Operations trait"
    }

    #[no_dispatch]
    fn describe(&self) -> String {
        format!("Value is {}", self.compute())
    }

    fn double(&self) -> i32 {
        self.compute() * 2
    }
}

#[tagged_dispatch(Operations)]
enum Calculator {
    Adder(Adder),
    Multiplier(Multiplier),
}

#[derive(Clone)]
struct Adder {
    a: i32,
    b: i32,
}

impl Operations for Adder {
    fn compute(&self) -> i32 {
        self.a + self.b
    }

    fn describe(&self) -> String {
        format!("Adder: {} + {} = {}", self.a, self.b, self.compute())
    }
}

#[derive(Clone)]
struct Multiplier {
    a: i32,
    b: i32,
}

impl Operations for Multiplier {
    fn compute(&self) -> i32 {
        self.a * self.b
    }

    fn double(&self) -> i32 {
        self.compute() * 2
    }
}

#[test]
fn test_no_dispatch_static() {
    assert_eq!(<Adder as Operations>::static_info(), "Operations trait");
    assert_eq!(Adder::static_info(), "Operations trait");
    assert_eq!(Multiplier::static_info(), "Operations trait");
}

#[test]
fn test_dispatched_methods() {
    let adder = Calculator::adder(Adder { a: 3, b: 4 });
    let mult = Calculator::multiplier(Multiplier { a: 5, b: 6 });

    assert_eq!(adder.compute(), 7);
    assert_eq!(mult.compute(), 30);

    assert_eq!(adder.double(), 14);
    assert_eq!(mult.double(), 60);
}

#[test]
fn test_no_dispatch_instance_method() {
    let adder = Adder { a: 10, b: 20 };
    let mult = Multiplier { a: 3, b: 7 };

    assert_eq!(adder.describe(), "Adder: 10 + 20 = 30");
    assert_eq!(mult.describe(), "Value is 21");
}

#[cfg(feature = "allocator-bumpalo")]
#[test]
fn test_no_dispatch_with_arena() {
    #[tagged_dispatch(Operations)]
    enum CalculatorArena<'a> {
        Adder(Adder),
        Multiplier(Multiplier),
    }

    let builder = CalculatorArena::arena_builder();

    let adder = builder.adder(Adder { a: 2, b: 3 });
    let mult = builder.multiplier(Multiplier { a: 4, b: 5 });

    assert_eq!(adder.compute(), 5);
    assert_eq!(mult.compute(), 20);

    assert_eq!(<Adder as Operations>::static_info(), "Operations trait");
}

#[tagged_dispatch]
trait MixedDispatch {
    fn dispatched_one(&self) -> i32;

    #[no_dispatch]
    fn not_dispatched(&self) -> bool {
        true
    }

    fn dispatched_two(&self) -> String;

    #[no_dispatch]
    fn also_not_dispatched(&self) -> &str {
        "default"
    }
}

#[tagged_dispatch(MixedDispatch)]
enum MixedType {
    TypeA(TypeA),
    TypeB(TypeB),
}

#[derive(Clone)]
struct TypeA {
    value: i32,
}

impl MixedDispatch for TypeA {
    fn dispatched_one(&self) -> i32 {
        self.value
    }

    fn dispatched_two(&self) -> String {
        format!("TypeA: {}", self.value)
    }

    fn not_dispatched(&self) -> bool {
        false
    }
}

#[derive(Clone)]
struct TypeB {
    text: String,
}

impl MixedDispatch for TypeB {
    fn dispatched_one(&self) -> i32 {
        self.text.len() as i32
    }

    fn dispatched_two(&self) -> String {
        format!("TypeB: {}", self.text)
    }
}

#[test]
fn test_mixed_dispatch() {
    let type_a = MixedType::type_a(TypeA { value: 42 });
    let type_b = MixedType::type_b(TypeB { text: "hello".to_string() });

    assert_eq!(type_a.dispatched_one(), 42);
    assert_eq!(type_b.dispatched_one(), 5);

    assert_eq!(type_a.dispatched_two(), "TypeA: 42");
    assert_eq!(type_b.dispatched_two(), "TypeB: hello");

    let concrete_a = TypeA { value: 100 };
    let concrete_b = TypeB { text: "test".to_string() };

    assert!(!concrete_a.not_dispatched());
    assert!(concrete_b.not_dispatched());

    assert_eq!(concrete_a.also_not_dispatched(), "default");
    assert_eq!(concrete_b.also_not_dispatched(), "default");
}