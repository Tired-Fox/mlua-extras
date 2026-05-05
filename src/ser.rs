use serde::ser::{
    self, Error as _, Serialize, SerializeMap, SerializeSeq, SerializeStruct, Serializer,
};
use std::fmt::{self, Write};

#[derive(Debug)]
pub struct Error(String);

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// Convert any type that is Serializable into it's lua value representation
pub fn to_lua_repr<T: Serialize>(value: &T) -> Result<String, Error> {
    let mut out = String::new();
    value.serialize(&mut LuaSerializer { out: &mut out })?;
    Ok(out)
}

struct LuaSerializer<'a> {
    out: &'a mut String,
}

impl<'a, 'b> Serializer for &'a mut LuaSerializer<'b> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = LuaSeq<'a, 'b>;
    type SerializeTuple = LuaSeq<'a, 'b>;
    type SerializeTupleStruct = LuaSeq<'a, 'b>;
    type SerializeTupleVariant = LuaSeq<'a, 'b>;
    type SerializeMap = LuaMap<'a, 'b>;
    type SerializeStruct = LuaMap<'a, 'b>;
    type SerializeStructVariant = LuaMap<'a, 'b>;

    fn serialize_bool(self, v: bool) -> Result<(), Error> {
        self.out.push_str(if v { "true" } else { "false" });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<(), Error> {
        write!(self.out, "{v}").map_err(Error::custom)
    }
    fn serialize_i16(self, v: i16) -> Result<(), Error> {
        write!(self.out, "{v}").map_err(Error::custom)
    }
    fn serialize_i32(self, v: i32) -> Result<(), Error> {
        write!(self.out, "{v}").map_err(Error::custom)
    }
    fn serialize_i64(self, v: i64) -> Result<(), Error> {
        write!(self.out, "{v}").map_err(Error::custom)
    }

    fn serialize_u8(self, v: u8) -> Result<(), Error> {
        write!(self.out, "{v}").map_err(Error::custom)
    }
    fn serialize_u16(self, v: u16) -> Result<(), Error> {
        write!(self.out, "{v}").map_err(Error::custom)
    }
    fn serialize_u32(self, v: u32) -> Result<(), Error> {
        write!(self.out, "{v}").map_err(Error::custom)
    }
    fn serialize_u64(self, v: u64) -> Result<(), Error> {
        write!(self.out, "{v}").map_err(Error::custom)
    }

    fn serialize_f32(self, v: f32) -> Result<(), Error> {
        if v.is_nan() {
            self.out.push_str("0/0");
        } else if v.is_infinite() {
            if v.is_sign_positive() {
                self.out.push_str("math.huge");
            } else {
                self.out.push_str("-math.huge");
            }
        } else {
            let mut buf = ryu::Buffer::new();
            let s = buf.format_finite(v);

            if s.contains('.') {
                self.out.push_str(s);
            } else {
                let s = format!("{s}.0");
                self.out.push_str(&s);
            }
        }

        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<(), Error> {
        let roundedf32 = v as f32;
        if (roundedf32 as f64) == v {
            self.serialize_f32(roundedf32)?;
        } else {
            if v.is_nan() {
                self.out.push_str("0/0");
            } else if v.is_infinite() {
                if v.is_sign_positive() {
                    self.out.push_str("math.huge");
                } else {
                    self.out.push_str("-math.huge");
                }
            } else {
                let mut buf = ryu::Buffer::new();
                let s = buf.format_finite(v);

                if s.contains('.') {
                    self.out.push_str(s);
                } else {
                    let s = format!("{s}.0");
                    self.out.push_str(&s);
                }
            }
        }

        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<(), Error> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<(), Error> {
        self.out.push('"');
        for ch in v.chars() {
            match ch {
                '\\' => self.out.push_str("\\\\"),
                '"' => self.out.push_str("\\\""),
                '\n' => self.out.push_str("\\n"),
                '\r' => self.out.push_str("\\r"),
                '\t' => self.out.push_str("\\t"),
                c => self.out.push(c),
            }
        }
        self.out.push('"');
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<(), Error> {
        let mut seq = self.serialize_seq(Some(v.len()))?;
        for b in v {
            seq.serialize_element(b)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<(), Error> {
        self.out.push_str("nil");
        Ok(())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<(), Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), Error> {
        self.out.push_str("nil");
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), Error> {
        self.out.push_str("nil");
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<(), Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.out.push('{');
        write!(self.out, "{} = ", lua_ident_or_bracket(variant)).map_err(Error::custom)?;
        value.serialize(&mut *self)?;
        self.out.push('}');
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        self.out.push('{');
        Ok(LuaSeq {
            ser: self,
            first: true,
            closes_twice: false,
        })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        self.serialize_seq(None)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        self.serialize_seq(None)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        self.out.push('{');
        write!(self.out, "{} = ", lua_ident_or_bracket(variant)).map_err(Error::custom)?;
        self.out.push('{');
        Ok(LuaSeq {
            ser: self,
            first: true,
            closes_twice: true,
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        self.out.push('{');
        Ok(LuaMap {
            ser: self,
            first: true,
            closes_twice: false,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        self.serialize_map(None)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        self.out.push('{');
        write!(self.out, "{} = {{", lua_ident_or_bracket(variant)).map_err(Error::custom)?;
        Ok(LuaMap {
            ser: self,
            first: true,
            closes_twice: true,
        })
    }
}

struct LuaSeq<'a, 'b> {
    ser: &'a mut LuaSerializer<'b>,
    first: bool,
    closes_twice: bool,
}

impl<'a, 'b> LuaSeq<'a, 'b> {
    fn comma(&mut self) {
        if !self.first {
            self.ser.out.push_str(", ");
        }
        self.first = false;
    }
}

impl<'a, 'b> SerializeSeq for LuaSeq<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        self.comma();
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<(), Error> {
        self.ser.out.push('}');
        if self.closes_twice {
            self.ser.out.push('}');
        }
        Ok(())
    }
}

impl<'a, 'b> ser::SerializeTuple for LuaSeq<'a, 'b> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<(), Error> {
        SerializeSeq::end(self)
    }
}

impl<'a, 'b> ser::SerializeTupleStruct for LuaSeq<'a, 'b> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<(), Error> {
        SerializeSeq::end(self)
    }
}

impl<'a, 'b> ser::SerializeTupleVariant for LuaSeq<'a, 'b> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<(), Error> {
        SerializeSeq::end(self)
    }
}

struct LuaMap<'a, 'b> {
    ser: &'a mut LuaSerializer<'b>,
    first: bool,
    closes_twice: bool,
}

impl<'a, 'b> LuaMap<'a, 'b> {
    fn comma(&mut self) {
        if !self.first {
            self.ser.out.push_str(", ");
        }
        self.first = false;
    }
}

impl<'a, 'b> SerializeMap for LuaMap<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Error> {
        self.comma();
        key.serialize(KeySerializer { out: self.ser.out })?;
        self.ser.out.push_str(" = ");
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<(), Error> {
        self.ser.out.push('}');
        if self.closes_twice {
            self.ser.out.push('}');
        }
        Ok(())
    }
}

impl<'a, 'b> SerializeStruct for LuaMap<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.comma();
        self.ser.out.push_str(lua_ident_or_bracket(key));
        self.ser.out.push_str(" = ");
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<(), Error> {
        SerializeMap::end(self)
    }
}

impl<'a, 'b> ser::SerializeStructVariant for LuaMap<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<(), Error> {
        SerializeMap::end(self)
    }
}

struct KeySerializer<'a> {
    out: &'a mut String,
}

impl<'a> Serializer for KeySerializer<'a> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = ser::Impossible<(), Error>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = ser::Impossible<(), Error>;
    type SerializeStruct = ser::Impossible<(), Error>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    fn serialize_str(self, v: &str) -> Result<(), Error> {
        self.out.push_str(lua_ident_or_bracket(v));
        Ok(())
    }

    fn serialize_bool(self, v: bool) -> Result<(), Error> {
        write!(self.out, "[{}]", if v { "true" } else { "false" }).map_err(Error::custom)
    }

    fn serialize_i64(self, v: i64) -> Result<(), Error> {
        write!(self.out, "[{v}]").map_err(Error::custom)
    }

    fn serialize_u64(self, v: u64) -> Result<(), Error> {
        write!(self.out, "[{v}]").map_err(Error::custom)
    }

    fn serialize_unit(self) -> Result<(), Error> {
        Err(Error::custom("unit cannot be a Lua table key"))
    }

    fn serialize_none(self) -> Result<(), Error> {
        Err(Error::custom("nil cannot be a Lua table key"))
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<(), Error> {
        value.serialize(self)
    }

    fn serialize_i8(self, v: i8) -> Result<(), Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i16(self, v: i16) -> Result<(), Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i32(self, v: i32) -> Result<(), Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_u8(self, v: u8) -> Result<(), Error> {
        self.serialize_u64(v as u64)
    }
    fn serialize_u16(self, v: u16) -> Result<(), Error> {
        self.serialize_u64(v as u64)
    }
    fn serialize_u32(self, v: u32) -> Result<(), Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_f32(self, _v: f32) -> Result<(), Error> {
        Err(Error::custom("float keys not supported"))
    }
    fn serialize_f64(self, _v: f64) -> Result<(), Error> {
        Err(Error::custom("float keys not supported"))
    }
    fn serialize_char(self, v: char) -> Result<(), Error> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<(), Error> {
        Err(Error::custom("bytes keys not supported"))
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Error> {
        self.serialize_unit()
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<(), Error> {
        self.serialize_str(variant)
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<(), Error> {
        Err(Error::custom("complex keys not supported"))
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Err(Error::custom("complex keys not supported"))
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Error> {
        Err(Error::custom("complex keys not supported"))
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Err(Error::custom("complex keys not supported"))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Err(Error::custom("complex keys not supported"))
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Err(Error::custom("complex keys not supported"))
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeStruct, Error> {
        Err(Error::custom("complex keys not supported"))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Err(Error::custom("complex keys not supported"))
    }
}

fn lua_ident_or_bracket(s: &str) -> &str {
    if is_lua_ident(s) {
        s
    } else {
        // This helper returns only &str, so callers that need brackets for
        // arbitrary strings should handle that separately if needed.
        // For simple struct fields, keep them identifier-safe or rename them.
        panic!("non-identifier key: {s}");
    }
}

fn is_lua_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use serde::Serialize;

    use super::*;

    /// Expect {type} with {value} to serialize to {expected}
    ///
    /// `expect!(type, &value, expected)`
    ///
    /// ```
    /// exptect!(u8, &3, "3");
    /// ```
    macro_rules! expect {
        ( $t:ty, $value:expr, $expected:literal ) => {
            let result = to_lua_repr::<$t>($value).unwrap();
            assert_eq!(result, $expected);
        };
    }

    #[test]
    fn test_rust_literal_values() {
        expect!(u8, &3, "3");
        expect!(u16, &3, "3");
        expect!(u32, &3, "3");
        expect!(u64, &3, "3");
        expect!(i8, &3, "3");
        expect!(i16, &3, "3");
        expect!(i32, &3, "3");
        expect!(i64, &3, "3");
        expect!(f32, &3.0, "3.0");
        expect!(f32, &3.1, "3.1");
        expect!(f64, &3.0, "3.0");
        expect!(f64, &3.1, "3.1");

        expect!(&str, &"test", "\"test\"");
        expect!(String, &"test".to_string(), "\"test\"");

        expect!(bool, &true, "true");
        expect!(bool, &false, "false");
    }

    #[test]
    fn test_rust_builtin_types() {
        expect!(Option<bool>, &None, "nil");
        expect!(Option<bool>, &Some(true), "true");
        expect!(&[u8], &b"test".as_slice(), "{116, 101, 115, 116}");
        expect!(BTreeMap<&str, bool>, &BTreeMap::from([("test", false), ("test2", true)]), "{test = false, test2 = true}");
        expect!(
            Vec<&str>,
            &Vec::from(["test", "test2"]),
            "{\"test\", \"test2\"}"
        );
    }

    #[derive(Serialize)]
    struct Person {
        name: String,
        age: usize,
    }

    #[test]
    fn test_rust_custom_types() {
        expect!(
            Person,
            &Person {
                name: "Test".into(),
                age: 10
            },
            "{name = \"Test\", age = 10}"
        );
    }
}
