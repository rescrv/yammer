use std::fmt::Display;

use serde::ser::Impossible;

struct AlmostXml {
    output: String,
    trailers: Vec<String>,
}

#[derive(Debug)]
pub struct XmlError(String);

impl Display for XmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "xml error: {}", self.0)
    }
}

impl<S: Into<String>> From<S> for XmlError {
    fn from(s: S) -> Self {
        Self(s.into())
    }
}

impl std::error::Error for XmlError {}

impl serde::ser::Error for XmlError {
    fn custom<T>(t: T) -> Self
    where
        T: std::fmt::Display,
    {
        XmlError(t.to_string())
    }
}

impl<'a> serde::ser::Serializer for &'a mut AlmostXml {
    type Ok = Self;
    type Error = XmlError;

    type SerializeSeq = &'a mut AlmostXml;
    type SerializeTuple = Impossible<Self, XmlError>;
    type SerializeTupleStruct = Impossible<Self, XmlError>;
    type SerializeTupleVariant = Impossible<Self, XmlError>;
    type SerializeMap = Impossible<Self, XmlError>;
    type SerializeStruct = &'a mut AlmostXml;
    type SerializeStructVariant = Impossible<Self, XmlError>;

    fn serialize_bool(self, b: bool) -> Result<Self, XmlError> {
        self.output += if b { "true\n" } else { "false\n" };
        Ok(self)
    }

    fn serialize_i8(self, v: i8) -> Result<Self, XmlError> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self, XmlError> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self, XmlError> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self, XmlError> {
        self.output += &format!("{}\n", v);
        Ok(self)
    }

    fn serialize_u8(self, v: u8) -> Result<Self, XmlError> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self, XmlError> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self, XmlError> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self, XmlError> {
        self.output += &format!("{}\n", v);
        Ok(self)
    }

    fn serialize_f32(self, v: f32) -> Result<Self, XmlError> {
        self.output += &format!("{}\n", v);
        Ok(self)
    }

    fn serialize_f64(self, v: f64) -> Result<Self, XmlError> {
        self.output += &format!("{}\n", v);
        Ok(self)
    }

    fn serialize_char(self, v: char) -> Result<Self, XmlError> {
        self.output.push(v);
        Ok(self)
    }

    fn serialize_str(self, v: &str) -> Result<Self, XmlError> {
        self.output.push_str(v);
        Ok(self)
    }

    fn serialize_bytes(self, _: &[u8]) -> Result<Self, XmlError> {
        Err(XmlError::from("serialize bytes not supported"))
    }

    fn serialize_none(self) -> Result<Self, XmlError> {
        Ok(self)
    }

    fn serialize_some<T>(self, v: &T) -> Result<Self, XmlError>
    where
        T: ?Sized + serde::Serialize,
    {
        v.serialize(&mut *self)?;
        if !self.output.ends_with("\n") {
            self.output.push('\n');
        }
        Ok(self)
    }

    fn serialize_unit(self) -> Result<Self, XmlError> {
        Ok(self)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self, XmlError> {
        self.output += &format!("<{name}></{name}>\n");
        Ok(self)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<Self, XmlError> {
        self.output += &format!("{}<{name}><{variant}></{variant}></{name}>\n", self.output);
        Ok(self)
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<Self, XmlError>
    where
        T: ?Sized + serde::Serialize,
    {
        self.output += &format!("<{name}>");
        value.serialize(&mut *self)?;
        self.output += &format!("</{name}>");
        Ok(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<Self, XmlError>
    where
        T: ?Sized + serde::Serialize,
    {
        Err(XmlError::from("serialize newtype-variant not supported"))
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<&'a mut AlmostXml, XmlError> {
        Ok(self)
    }

    fn serialize_tuple(self, _: usize) -> Result<Impossible<Self, XmlError>, XmlError> {
        Err(XmlError::from("serialize tuple not supported"))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _: usize,
    ) -> Result<Impossible<Self, XmlError>, XmlError> {
        Err(XmlError::from("serialize tuple-struct not supported"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _: &'static str,
        _len: usize,
    ) -> Result<Impossible<Self, XmlError>, XmlError> {
        Err(XmlError::from("serialize tuple-variant not supported"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Impossible<Self, XmlError>, XmlError> {
        Err(XmlError::from("serialize map not supported"))
    }

    fn serialize_struct(self, name: &'static str, _: usize) -> Result<&'a mut AlmostXml, XmlError> {
        self.output += &format!("<{name}>\n");
        self.trailers.push(format!("</{name}>\n"));
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Impossible<Self, XmlError>, XmlError> {
        Err(XmlError::from("serialize struct variant not supported"))
    }
}

impl<'a> serde::ser::SerializeSeq for &'a mut AlmostXml {
    type Ok = &'a mut AlmostXml;
    type Error = XmlError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), XmlError>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(&mut **self)?;
        if !self.output.ends_with("\n") {
            self.output.push('\n');
        }
        Ok(())
    }

    fn end(self) -> Result<&'a mut AlmostXml, XmlError> {
        Ok(self)
    }
}

impl<'a> serde::ser::SerializeStruct for &'a mut AlmostXml {
    type Ok = &'a mut AlmostXml;
    type Error = XmlError;

    fn serialize_field<T>(&mut self, name: &'static str, value: &T) -> Result<(), XmlError>
    where
        T: ?Sized + serde::Serialize,
    {
        self.output += &format!("<{name}>\n");
        value.serialize(&mut **self)?;
        if !self.output.ends_with("\n") {
            self.output.push('\n');
        }
        self.output += &format!("</{name}>\n");
        Ok(())
    }

    fn end(self) -> Result<&'a mut AlmostXml, XmlError> {
        if let Some(trailer) = self.trailers.pop() {
            self.output.push_str(&trailer);
        }
        Ok(self)
    }
}

/// Formattable requires that a type implement Display and serde::Serialize.  In exchange, it can be
/// put into a prompt in plaintext, json, or xml form.
pub trait Formattable: Display + serde::Serialize {
    /// Return white-spaced JSON suitable for the prompt.
    fn as_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Return plain text.
    fn as_text(&self) -> String {
        self.to_string()
    }

    /// Return pseudo-XML.
    fn as_xml(&self) -> Result<String, XmlError> {
        let almost_xml = &mut AlmostXml {
            output: String::new(),
            trailers: vec![],
        };
        let almost_xml = self.serialize(almost_xml)?;
        while let Some(trailer) = almost_xml.trailers.pop() {
            almost_xml.output.push_str(&trailer);
        }
        Ok(std::mem::take(&mut almost_xml.output))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::approx_constant)]
    use super::*;

    #[derive(serde::Deserialize, serde::Serialize)]
    struct TestFormattable {
        x: String,
        pi: f64,
        e: f64,
    }

    impl Display for TestFormattable {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            writeln!(f, "{}\npi: {}\ne: {}", self.x, self.pi, self.e)
        }
    }

    impl Formattable for TestFormattable {}

    // json
    #[test]
    fn json() {
        assert_eq!(
            r#"{
  "x": "e^pi i",
  "pi": 3.14159,
  "e": 2.71828
}"#,
            TestFormattable {
                x: "e^pi i".to_string(),
                pi: 3.14159,
                e: 2.71828,
            }
            .as_json()
            .unwrap()
        );
    }

    // text
    #[test]
    fn text() {
        assert_eq!(
            r#"e^pi i
pi: 3.14159
e: 2.71828
"#,
            TestFormattable {
                x: "e^pi i".to_string(),
                pi: 3.14159,
                e: 2.71828,
            }
            .as_text()
        );
    }

    // xml
    #[test]
    fn xml() {
        assert_eq!(
            r#"<TestFormattable>
<x>
e^pi i
</x>
<pi>
3.14159
</pi>
<e>
2.71828
</e>
</TestFormattable>
"#,
            TestFormattable {
                x: "e^pi i".to_string(),
                pi: 3.14159,
                e: 2.71828,
            }
            .as_xml()
            .unwrap()
        );
    }

    #[derive(serde::Deserialize, serde::Serialize)]
    #[serde(rename = "example")]
    struct Example {
        input: String,
        output: String,
    }

    #[derive(serde::Deserialize, serde::Serialize)]
    struct SamplePrompt {
        instructions: Vec<String>,
        examples: Vec<Example>,
    }

    impl Display for SamplePrompt {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            for instruction in self.instructions.iter() {
                writeln!(f, "- {}", instruction)?;
            }
            if !self.instructions.is_empty() {
                writeln!(f)?;
            }
            for (idx, example) in self.examples.iter().enumerate() {
                writeln!(f, "**Example {}**\n", idx + 1)?;
                writeln!(f, "Input: {}", example.input)?;
                writeln!(f, "Output: {}\n", example.output)?;
            }
            Ok(())
        }
    }

    impl Formattable for SamplePrompt {}

    #[test]
    fn json_prompt() {
        assert_eq!(
            r#"{
  "instructions": [
    "foo",
    "bar"
  ],
  "examples": [
    {
      "input": "a",
      "output": "A"
    },
    {
      "input": "1",
      "output": "!"
    },
    {
      "input": "z",
      "output": "Z"
    },
    {
      "input": "/",
      "output": "?"
    }
  ]
}"#,
            SamplePrompt {
                instructions: vec!["foo".to_string(), "bar".to_string()],
                examples: vec![
                    Example {
                        input: "a".to_string(),
                        output: "A".to_string(),
                    },
                    Example {
                        input: "1".to_string(),
                        output: "!".to_string(),
                    },
                    Example {
                        input: "z".to_string(),
                        output: "Z".to_string(),
                    },
                    Example {
                        input: "/".to_string(),
                        output: "?".to_string(),
                    },
                ],
            }
            .as_json()
            .unwrap()
        );
    }

    #[test]
    fn text_prompt() {
        assert_eq!(
            r#"- foo
- bar

**Example 1**

Input: a
Output: A

**Example 2**

Input: 1
Output: !

**Example 3**

Input: z
Output: Z

**Example 4**

Input: /
Output: ?

"#,
            SamplePrompt {
                instructions: vec!["foo".to_string(), "bar".to_string()],
                examples: vec![
                    Example {
                        input: "a".to_string(),
                        output: "A".to_string(),
                    },
                    Example {
                        input: "1".to_string(),
                        output: "!".to_string(),
                    },
                    Example {
                        input: "z".to_string(),
                        output: "Z".to_string(),
                    },
                    Example {
                        input: "/".to_string(),
                        output: "?".to_string(),
                    },
                ],
            }
            .as_text()
        );
    }

    #[test]
    fn xml_prompt() {
        assert_eq!(
            r#"<SamplePrompt>
<instructions>
foo
bar
</instructions>
<examples>
<example>
<input>
a
</input>
<output>
A
</output>
</example>
<example>
<input>
1
</input>
<output>
!
</output>
</example>
<example>
<input>
z
</input>
<output>
Z
</output>
</example>
<example>
<input>
/
</input>
<output>
?
</output>
</example>
</examples>
</SamplePrompt>
"#,
            SamplePrompt {
                instructions: vec!["foo".to_string(), "bar".to_string()],
                examples: vec![
                    Example {
                        input: "a".to_string(),
                        output: "A".to_string(),
                    },
                    Example {
                        input: "1".to_string(),
                        output: "!".to_string(),
                    },
                    Example {
                        input: "z".to_string(),
                        output: "Z".to_string(),
                    },
                    Example {
                        input: "/".to_string(),
                        output: "?".to_string(),
                    },
                ],
            }
            .as_xml()
            .unwrap()
        );
    }
}
