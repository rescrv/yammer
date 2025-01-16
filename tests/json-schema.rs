use yammer::JsonSchema;

#[allow(dead_code)]
#[test]
fn json_schema() {
    #[derive(yammer_derive::JsonSchema)]
    struct MyStruct {
        pub foo: f64,
        pub bar: String,
        pub baz: Option<String>,
        pub quux: Vec<String>,
    }
    assert_eq!(
        r#"{
  "properties": {
    "bar": {
      "type": "string"
    },
    "baz": {
      "nullable": true,
      "type": "string"
    },
    "foo": {
      "type": "number"
    },
    "quux": {
      "items": {
        "type": "string"
      },
      "type": "array"
    }
  },
  "required": [
    "foo",
    "bar",
    "baz",
    "quux"
  ],
  "type": "object"
}"#
        .to_string(),
        serde_json::to_string_pretty(&<MyStruct as JsonSchema>::json_schema()).unwrap(),
    );
}
