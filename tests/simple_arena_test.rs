#![cfg(feature = "allocator-bumpalo")]

use tagged_dispatch::tagged_dispatch;

#[tagged_dispatch]
trait Simple {
    fn get(&self) -> i32;
}

#[tagged_dispatch(Simple)]
enum Value<'a> {
    A(A),
    B(B),
}

struct A {
    val: i32,
}

impl Simple for A {
    fn get(&self) -> i32 {
        self.val
    }
}

struct B {
    val: i32,
}

impl Simple for B {
    fn get(&self) -> i32 {
        self.val * 2
    }
}

#[test]
fn test_basic_arena() {
    let builder = Value::arena_builder();
    let a = builder.a(A { val: 10 });
    let b = builder.b(B { val: 5 });

    assert_eq!(a.get(), 10);
    assert_eq!(b.get(), 10);

    // Test that it's Copy
    let a2 = a;
    assert_eq!(a.get(), a2.get());
}