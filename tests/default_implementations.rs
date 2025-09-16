use tagged_dispatch::tagged_dispatch;

#[tagged_dispatch]
trait Animal {
    fn make_sound(&self) -> &str;

    fn legs(&self) -> u32 {
        4
    }

    fn can_fly(&self) -> bool {
        false
    }

    fn habitat(&self) -> &str {
        "land"
    }

    fn description(&self) -> String {
        format!(
            "A {} with {} legs that lives on {}",
            self.make_sound(),
            self.legs(),
            self.habitat()
        )
    }
}

#[tagged_dispatch(Animal)]
enum Pet {
    Dog,     // Simplified syntax
    Cat,     // Macro expands these to Dog(Dog), Cat(Cat), etc.
    Bird,
    Spider,
}

#[derive(Clone)]
struct Dog {
    breed: String,
}

impl Animal for Dog {
    fn make_sound(&self) -> &str {
        "woof"
    }
}

#[derive(Clone)]
struct Cat {
    name: String,
}

impl Animal for Cat {
    fn make_sound(&self) -> &str {
        "meow"
    }

    fn description(&self) -> String {
        format!("A cat named {} that says {}", self.name, self.make_sound())
    }
}

#[derive(Clone)]
struct Bird {
    species: String,
}

impl Animal for Bird {
    fn make_sound(&self) -> &str {
        "chirp"
    }

    fn legs(&self) -> u32 {
        2
    }

    fn can_fly(&self) -> bool {
        true
    }

    fn habitat(&self) -> &str {
        "air"
    }
}

#[derive(Clone)]
struct Spider {
    venomous: bool,
}

impl Animal for Spider {
    fn make_sound(&self) -> &str {
        "silent"
    }

    fn legs(&self) -> u32 {
        8
    }

    fn description(&self) -> String {
        format!(
            "A {} spider with {} legs",
            if self.venomous { "venomous" } else { "harmless" },
            self.legs()
        )
    }
}

#[test]
fn test_default_implementations() {
    let dog = Pet::dog(Dog {
        breed: "Labrador".to_string(),
    });
    let cat = Pet::cat(Cat {
        name: "Whiskers".to_string(),
    });
    let bird = Pet::bird(Bird {
        species: "Sparrow".to_string(),
    });
    let spider = Pet::spider(Spider { venomous: false });

    assert_eq!(dog.make_sound(), "woof");
    assert_eq!(dog.legs(), 4);
    assert!(!dog.can_fly());
    assert_eq!(dog.habitat(), "land");
    assert_eq!(dog.description(), "A woof with 4 legs that lives on land");

    assert_eq!(cat.make_sound(), "meow");
    assert_eq!(cat.legs(), 4);
    assert!(!cat.can_fly());
    assert_eq!(cat.habitat(), "land");
    assert_eq!(cat.description(), "A cat named Whiskers that says meow");

    assert_eq!(bird.make_sound(), "chirp");
    assert_eq!(bird.legs(), 2);
    assert!(bird.can_fly());
    assert_eq!(bird.habitat(), "air");
    assert_eq!(bird.description(), "A chirp with 2 legs that lives on air");

    assert_eq!(spider.make_sound(), "silent");
    assert_eq!(spider.legs(), 8);
    assert!(!spider.can_fly());
    assert_eq!(spider.habitat(), "land");
    assert_eq!(spider.description(), "A harmless spider with 8 legs");
}

#[test]
fn test_all_defaults() {
    #[tagged_dispatch]
    trait Defaultable {
        fn required(&self) -> i32;

        fn optional_one(&self) -> i32 {
            100
        }

        fn optional_two(&self) -> &str {
            "default"
        }

        fn optional_three(&self) -> bool {
            true
        }
    }

    #[tagged_dispatch(Defaultable)]
    enum Thing {
        Simple(Simple),
        Complex(Complex),
    }

    #[derive(Clone)]
    struct Simple {
        value: i32,
    }

    impl Defaultable for Simple {
        fn required(&self) -> i32 {
            self.value
        }
    }

    #[derive(Clone)]
    struct Complex {
        value: i32,
    }

    impl Defaultable for Complex {
        fn required(&self) -> i32 {
            self.value * 2
        }

        fn optional_one(&self) -> i32 {
            200
        }

        fn optional_two(&self) -> &str {
            "complex"
        }
    }

    let simple = Thing::simple(Simple { value: 5 });
    let complex = Thing::complex(Complex { value: 10 });

    assert_eq!(simple.required(), 5);
    assert_eq!(simple.optional_one(), 100);
    assert_eq!(simple.optional_two(), "default");
    assert!(simple.optional_three());

    assert_eq!(complex.required(), 20);
    assert_eq!(complex.optional_one(), 200);
    assert_eq!(complex.optional_two(), "complex");
    assert!(complex.optional_three());
}

#[cfg(feature = "allocator-bumpalo")]
#[test]
fn test_defaults_with_arena() {
    #[tagged_dispatch(Animal)]
    enum PetArena<'a> {
        Dog(Dog),
        Cat(Cat),
        Bird(Bird),
        Spider(Spider),
    }

    let builder = PetArena::arena_builder();

    let dog = builder.dog(Dog {
        breed: "Poodle".to_string(),
    });
    let bird = builder.bird(Bird {
        species: "Eagle".to_string(),
    });

    assert_eq!(dog.make_sound(), "woof");
    assert_eq!(dog.legs(), 4);
    assert!(!dog.can_fly());

    assert_eq!(bird.make_sound(), "chirp");
    assert_eq!(bird.legs(), 2);
    assert!(bird.can_fly());

    let dog2 = dog;
    assert_eq!(dog.legs(), dog2.legs());
}

#[test]
fn test_enum_size() {
    assert_eq!(std::mem::size_of::<Pet>(), 8);
}