use tagged_dispatch::tagged_dispatch;

#[tagged_dispatch]
trait Renderer {
    fn render(&self) -> String;

    fn priority(&self) -> u32 {
        1
    }

    #[no_dispatch]
    fn renderer_type() -> &'static str {
        "Generic Renderer"
    }
}

#[tagged_dispatch]
trait Cacheable {
    fn cache_key(&self) -> String;

    fn ttl_seconds(&self) -> u64 {
        3600
    }
}

#[tagged_dispatch]
trait Validator {
    fn is_valid(&self) -> bool;

    #[no_dispatch]
    fn validation_rules() -> Vec<&'static str> {
        vec!["default_rule"]
    }

    fn error_message(&self) -> Option<String> {
        if !self.is_valid() {
            Some("Validation failed".to_string())
        } else {
            None
        }
    }
}

#[tagged_dispatch(Renderer, Cacheable, Validator)]
enum Component {
    Button,  // Simplified: expands to Button(Button)
    Input,   // Simplified: expands to Input(Input)
    Label,   // Simplified: expands to Label(Label)
}

#[derive(Clone)]
struct Button {
    text: String,
    enabled: bool,
}

impl Renderer for Button {
    fn render(&self) -> String {
        format!(
            "<button{}>{}</button>",
            if self.enabled { "" } else { " disabled" },
            self.text
        )
    }

    fn priority(&self) -> u32 {
        10
    }
}

impl Cacheable for Button {
    fn cache_key(&self) -> String {
        format!("button_{}", self.text)
    }
}

impl Validator for Button {
    fn is_valid(&self) -> bool {
        !self.text.is_empty()
    }

    fn error_message(&self) -> Option<String> {
        if self.text.is_empty() {
            Some("Button text cannot be empty".to_string())
        } else {
            None
        }
    }
}

#[derive(Clone)]
struct Input {
    name: String,
    value: String,
    max_length: usize,
}

impl Renderer for Input {
    fn render(&self) -> String {
        format!(
            r#"<input name="{}" value="{}" maxlength="{}"/>"#,
            self.name, self.value, self.max_length
        )
    }
}

impl Cacheable for Input {
    fn cache_key(&self) -> String {
        format!("input_{}_{}", self.name, self.value)
    }

    fn ttl_seconds(&self) -> u64 {
        300
    }
}

impl Validator for Input {
    fn is_valid(&self) -> bool {
        self.value.len() <= self.max_length
    }

    fn validation_rules() -> Vec<&'static str> {
        vec!["max_length", "required"]
    }
}

#[derive(Clone)]
struct Label {
    text: String,
}

impl Renderer for Label {
    fn render(&self) -> String {
        format!("<label>{}</label>", self.text)
    }

    fn priority(&self) -> u32 {
        0
    }
}

impl Cacheable for Label {
    fn cache_key(&self) -> String {
        format!("label_{}", self.text)
    }

    fn ttl_seconds(&self) -> u64 {
        86400
    }
}

impl Validator for Label {
    fn is_valid(&self) -> bool {
        true
    }
}

#[test]
fn test_mixed_traits_and_defaults() {
    let button = Component::button(Button {
        text: "Click me".to_string(),
        enabled: true,
    });

    let input = Component::input(Input {
        name: "username".to_string(),
        value: "john".to_string(),
        max_length: 10,
    });

    let label = Component::label(Label {
        text: "Name:".to_string(),
    });

    assert_eq!(button.render(), "<button>Click me</button>");
    assert_eq!(button.priority(), 10);
    assert_eq!(button.cache_key(), "button_Click me");
    assert_eq!(button.ttl_seconds(), 3600);
    assert!(button.is_valid());
    assert_eq!(button.error_message(), None);

    assert_eq!(
        input.render(),
        r#"<input name="username" value="john" maxlength="10"/>"#
    );
    assert_eq!(input.priority(), 1);
    assert_eq!(input.cache_key(), "input_username_john");
    assert_eq!(input.ttl_seconds(), 300);
    assert!(input.is_valid());
    assert_eq!(input.error_message(), None);

    assert_eq!(label.render(), "<label>Name:</label>");
    assert_eq!(label.priority(), 0);
    assert_eq!(label.cache_key(), "label_Name:");
    assert_eq!(label.ttl_seconds(), 86400);
    assert!(label.is_valid());
    assert_eq!(label.error_message(), None);
}

#[test]
fn test_invalid_states() {
    let invalid_button = Component::button(Button {
        text: String::new(),
        enabled: false,
    });

    let invalid_input = Component::input(Input {
        name: "description".to_string(),
        value: "This is way too long".to_string(),
        max_length: 5,
    });

    assert_eq!(invalid_button.render(), "<button disabled></button>");
    assert!(!invalid_button.is_valid());
    assert_eq!(
        invalid_button.error_message(),
        Some("Button text cannot be empty".to_string())
    );

    assert!(!invalid_input.is_valid());
    assert_eq!(
        invalid_input.error_message(),
        Some("Validation failed".to_string())
    );
}

#[test]
fn test_static_methods() {
    assert_eq!(<Button as Renderer>::renderer_type(), "Generic Renderer");
    assert_eq!(Button::renderer_type(), "Generic Renderer");
    assert_eq!(Input::renderer_type(), "Generic Renderer");

    assert_eq!(<Button as Validator>::validation_rules(), vec!["default_rule"]);
    assert_eq!(Button::validation_rules(), vec!["default_rule"]);
    assert_eq!(Input::validation_rules(), vec!["max_length", "required"]);
    assert_eq!(Label::validation_rules(), vec!["default_rule"]);
}

#[cfg(feature = "allocator-bumpalo")]
#[test]
fn test_mixed_with_arena() {
    #[tagged_dispatch(Renderer, Cacheable, Validator)]
    enum ComponentArena<'a> {
        Button,  // Arena version also uses simplified syntax
        Input,
        Label,
    }

    let builder = ComponentArena::arena_builder();

    let button = builder.button(Button {
        text: "Submit".to_string(),
        enabled: true,
    });

    let input = builder.input(Input {
        name: "email".to_string(),
        value: "test@example.com".to_string(),
        max_length: 50,
    });

    let label = builder.label(Label {
        text: "Email:".to_string(),
    });

    assert_eq!(button.render(), "<button>Submit</button>");
    assert_eq!(button.cache_key(), "button_Submit");
    assert!(button.is_valid());

    assert!(input.render().contains("email"));
    assert_eq!(input.ttl_seconds(), 300);
    assert!(input.is_valid());

    assert_eq!(label.render(), "<label>Email:</label>");
    assert_eq!(label.priority(), 0);

    let button2 = button;
    assert_eq!(button.priority(), button2.priority());
}

#[test]
fn test_collection_operations() {
    let components: Vec<Component> = vec![
        Component::button(Button {
            text: "Save".to_string(),
            enabled: true,
        }),
        Component::input(Input {
            name: "password".to_string(),
            value: "secret".to_string(),
            max_length: 20,
        }),
        Component::label(Label {
            text: "Password:".to_string(),
        }),
    ];

    let mut sorted = components
        .iter()
        .map(|c| (c.priority(), c.render()))
        .collect::<Vec<_>>();
    sorted.sort_by_key(|(p, _)| *p);

    assert_eq!(sorted[0].0, 0);
    assert!(sorted[0].1.contains("<label>"));
    assert_eq!(sorted[1].0, 1);
    assert!(sorted[1].1.contains("<input"));
    assert_eq!(sorted[2].0, 10);
    assert!(sorted[2].1.contains("<button>"));

    let cache_keys: Vec<String> = components.iter().map(|c| c.cache_key()).collect();
    assert_eq!(cache_keys.len(), 3);
    assert!(cache_keys[0].starts_with("button_"));
    assert!(cache_keys[1].starts_with("input_"));
    assert!(cache_keys[2].starts_with("label_"));

    let all_valid = components.iter().all(|c| c.is_valid());
    assert!(all_valid);
}

#[test]
fn test_enum_size() {
    assert_eq!(std::mem::size_of::<Component>(), 8);
}