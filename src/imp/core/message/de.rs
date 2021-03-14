use serde::{
    de,
    de::{IntoDeserializer, Visitor}
};
use serde_json::value::{Map, Value};
use std::convert::TryFrom;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0:}")]
    Msg(String),
    #[error("Not be able to deserialize from blank")]
    Blank,
    #[error("Incorrect type")]
    TypeMismatch,
    #[error("{0:} isn't supported")]
    NotSupported(&'static str),
    #[error(transparent)]
    Serde(#[from] serde_json::Error)
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display
    {
        Self::Msg(msg.to_string())
    }
}

pub(crate) struct Deserializer<'de> {
    input: &'de Value,
    stack: Vec<&'de Value>
}

impl<'de> Deserializer<'de> {
    fn new(v: &'de Value) -> Self {
        let mut stack = Vec::new();
        stack.push(v);
        Self { input: v, stack }
    }

    fn pop(&mut self) -> Result<&'de Value, Error> { self.stack.pop().ok_or(Error::Blank) }
}

pub(crate) fn from_value<T>(v: &Value) -> Result<T, Error>
where
    T: de::DeserializeOwned
{
    let mut deserializer = Deserializer::new(v);
    let t = T::deserialize(&mut deserializer)?;
    Ok(t)
}

macro_rules! int {
    ($t:ty, $base:ty) => {
        paste::paste! {
            fn [<deserialize_$t>]<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>
            {
                let v = self.pop()?;
                let i1 = v.as_object()
                    .and_then(|m| m.get("n"))
                    .and_then(|v| v.[<as_$base>]())
                    .ok_or(Error::TypeMismatch);
                let i2 = v.[<as_$base>]().ok_or(Error::TypeMismatch);
                let i = i1.or(i2)?;
                let i = $t::try_from(i).map_err(|_| Error::TypeMismatch)?;
                visitor.[<visit_$t>](i)
            }
        }
    };
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        log::trace!("any");
        let v = *self.stack.last().ok_or(Error::Blank)?;
        match v {
            Value::Null => return self.deserialize_unit(visitor),
            Value::Bool(_) => return self.deserialize_bool(visitor),
            Value::Number(x) if x.is_u64() => return self.deserialize_u64(visitor),
            Value::Number(x) if x.is_i64() => return self.deserialize_i64(visitor),
            Value::Number(x) if x.is_f64() => return self.deserialize_f64(visitor),
            Value::Number(_) => unreachable!(),
            Value::String(_) => return self.deserialize_str(visitor),
            Value::Array(_) => return self.deserialize_seq(visitor),
            Value::Object(m) => {
                log::trace!("object");
                if let Some(v) = m.get("v") {
                    return match v.as_str() {
                        Some("Infinity") | Some("-Infinity") | Some("-0") | Some("NaN") => {
                            self.deserialize_f64(visitor)
                        }
                        Some("undefined") | Some("null") => self.deserialize_unit(visitor),
                        _ => {
                            let _ = self.pop()?;
                            self.stack.push(v);
                            self.deserialize_any(visitor)
                        }
                    };
                } else if let Some(_a) = m.get("a") {
                    return self.deserialize_seq(visitor);
                } else if let Some(_d) = m.get("d") {
                    unimplemented!()
                } else if let Some(_o) = m.get("o") {
                    return self.deserialize_map(visitor);
                } else if let Some(n) = m.get("n") {
                    return match n {
                        Value::Number(x) if x.is_u64() => self.deserialize_u64(visitor),
                        Value::Number(x) if x.is_i64() => self.deserialize_i64(visitor),
                        Value::Number(x) if x.is_f64() => self.deserialize_f64(visitor),
                        _ => {
                            log::error!("{:?}", n);
                            Err(Error::TypeMismatch)
                        }
                    };
                } else if let Some(_s) = m.get("s") {
                    return self.deserialize_str(visitor);
                } else if let Some(_b) = m.get("b") {
                    return self.deserialize_bool(visitor);
                } else {
                    return self.deserialize_map(visitor);
                }
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        let v = self.pop()?;
        let b1 = v
            .as_object()
            .and_then(|m| m.get("b"))
            .and_then(|v| v.as_bool())
            .ok_or(Error::TypeMismatch);
        let b2 = v.as_bool().ok_or(Error::TypeMismatch);
        let b = b1.or(b2)?;
        visitor.visit_bool(b)
    }

    int! {i8, i64}
    int! {i16, i64}
    int! {i32, i64}
    int! {i64, i64}
    int! {u8, u64}
    int! {u16, u64}
    int! {u32, u64}
    int! {u64, u64}

    fn deserialize_char<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        Err(Error::NotSupported("deserialize_char"))
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        Err(Error::NotSupported("deserialize_bytes"))
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        Err(Error::NotSupported("deserialize_byte_buf"))
    }

    fn deserialize_f32<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        Err(Error::NotSupported("deserialize_f32"))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        let v = self.pop()?;
        let f1 = v
            .as_object()
            .and_then(|m| m.get("n"))
            .and_then(|v| v.as_f64())
            .ok_or(Error::TypeMismatch);
        let f2 = v.as_f64().ok_or(Error::TypeMismatch);
        let f3 = v
            .as_object()
            .and_then(|m| m.get("v"))
            .and_then(|v| v.as_str())
            .and_then(|s| match s {
                "Infinity" => Some(f64::INFINITY),
                "-Infinity" => Some(f64::NEG_INFINITY),
                "-0" => Some(-0.0),
                "NaN" => Some(f64::NAN),
                _ => None
            })
            .ok_or(Error::TypeMismatch);
        let f = f1.or(f2).or(f3)?;
        visitor.visit_f64(f)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        let v = self.pop()?;
        let s1 = v
            .as_object()
            .and_then(|m| m.get("s"))
            .and_then(|v| v.as_str())
            .ok_or(Error::TypeMismatch);
        let s2 = v.as_str().ok_or(Error::TypeMismatch);
        let s = s1.or(s2)?;
        visitor.visit_borrowed_str(s)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        let v = self.pop()?;
        let n1 = v
            .as_object()
            .and_then(|m| m.get("v"))
            .and_then(|v| v.as_str())
            .and_then(|s| match s {
                "undefined" => Some(()),
                "null" => Some(()),
                _ => None
            });
        let n2 = v.as_null();
        let is_null = n1.or(n2).is_some();
        if is_null {
            visitor.visit_none()
        } else {
            self.stack.push(v);
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        let v = self.pop()?;
        let n1 = v
            .as_object()
            .and_then(|m| m.get("v"))
            .and_then(|v| v.as_str())
            .and_then(|s| match s {
                "undefined" => Some(()),
                "null" => Some(()),
                _ => None
            });
        let n2 = v.as_null();
        let _ = n1.or(n2).ok_or(Error::TypeMismatch)?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        let v = self.pop()?;
        let a1 = v
            .as_object()
            .and_then(|m| m.get("a"))
            .and_then(|v| v.as_array())
            .ok_or(Error::TypeMismatch);
        let a2 = v.as_array().ok_or(Error::TypeMismatch);
        let a = a1.or(a2)?;
        visitor.visit_seq(Array::new(&mut self, a))
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        self.deserialize_seq(visitor)
    }

    // TODO: datetime
    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        log::trace!("{:?}", &self.stack);
        let v = self.pop()?;
        let m = v.as_object().ok_or(Error::TypeMismatch)?;
        let o1 = m
            .get("o")
            .and_then(|v| v.as_array())
            .ok_or(Error::TypeMismatch);
        if m.contains_key("v") || m.contains_key("a") || m.contains_key("d") {
            return Err(Error::TypeMismatch);
        } else if m.contains_key("o") {
            visitor.visit_map(ObjectArr::new(&mut self, o1?))
        } else if m.contains_key("n") || m.contains_key("s") || m.contains_key("b") {
            return Err(Error::TypeMismatch);
        } else {
            visitor.visit_map(Object::new(&mut self, m))
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        log::trace!("enum");
        let v = self.pop()?;
        match v {
            Value::String(s) => visitor.visit_enum(s.as_str().into_deserializer()),
            Value::Object(m) => visitor.visit_enum(Enum::new(self, m)),
            _ => Err(Error::TypeMismatch)
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        self.deserialize_any(visitor)
    }
}

struct Array<'a, 'de: 'a> {
    prime: &'a mut Deserializer<'de>,
    data: std::slice::Iter<'de, Value>
}

impl<'a, 'de> Array<'a, 'de> {
    fn new(prime: &'a mut Deserializer<'de>, arr: &'de [Value]) -> Self {
        log::trace!("{:?}", arr);
        Array {
            prime,
            data: arr.into_iter()
        }
    }
}

impl<'de, 'a> de::SeqAccess<'de> for Array<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>
    {
        let data = match self.data.next() {
            Some(x) => x,
            None => return Ok(None)
        };
        self.prime.stack.push(data);
        log::trace!("data");
        seed.deserialize(&mut *self.prime).map(Some)
    }
}

struct Object<'a, 'de: 'a> {
    prime: &'a mut Deserializer<'de>,
    keys: serde_json::map::Keys<'de>,
    values: serde_json::map::Values<'de>
}

impl<'a, 'de> Object<'a, 'de> {
    fn new(prime: &'a mut Deserializer<'de>, obj: &'de Map<String, Value>) -> Self {
        Self {
            prime,
            keys: obj.keys(),
            values: obj.values()
        }
    }
}

impl<'de, 'a> de::MapAccess<'de> for Object<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>
    {
        let data = match self.keys.next() {
            Some(x) => x,
            None => return Ok(None)
        };
        log::trace!("key");
        let mut d = serde_json::Deserializer::from_str(data);
        Ok(Some(seed.deserialize(&mut d)?))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>
    {
        let data = self.values.next().ok_or(Error::Blank)?;
        self.prime.stack.push(data);
        seed.deserialize(&mut *self.prime)
    }
}

struct ObjectArr<'a, 'de: 'a> {
    prime: &'a mut Deserializer<'de>,
    arr: &'de [Value],
    idx: usize
}

impl<'a, 'de> ObjectArr<'a, 'de> {
    fn new(prime: &'a mut Deserializer<'de>, arr: &'de [Value]) -> Self {
        Self { prime, arr, idx: 0 }
    }
}

impl<'de, 'a> de::MapAccess<'de> for ObjectArr<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>
    {
        let data = if self.idx < self.arr.len() {
            self.idx += 1;
            self.arr[self.idx - 1]
                .as_object()
                .and_then(|m| m.get("k"))
                .ok_or(Error::Blank)?
        } else {
            return Ok(None);
        };
        log::trace!("key {:?}", &data);
        self.prime.stack.push(data);
        Ok(Some(seed.deserialize(&mut *self.prime)?))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>
    {
        let data = self.arr[self.idx - 1]
            .as_object()
            .and_then(|m| m.get("v"))
            .ok_or(Error::Blank)?;
        log::trace!("value {:?}", &data);
        self.prime.stack.push(data);
        seed.deserialize(&mut *self.prime)
    }
}

struct Enum<'a, 'de: 'a> {
    prime: &'a mut Deserializer<'de>,
    map: &'de Map<String, Value>
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(prime: &'a mut Deserializer<'de>, map: &'de Map<String, Value>) -> Self {
        log::trace!("enum {:?}", map);
        Enum { prime, map }
    }
}

impl<'de, 'a> de::EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>
    {
        let s = self.map.keys().next().ok_or(Error::Blank)?;
        log::trace!("asdf");
        let mut d = serde_json::Deserializer::from_str(s);
        let v = seed.deserialize(&mut d)?;
        self.prime.stack.push(self.map.get(s).unwrap());
        Ok((v, self))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    fn unit_variant(self) -> Result<(), Self::Error> { unimplemented!() }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>
    {
        seed.deserialize(self.prime)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        de::Deserializer::deserialize_seq(self.prime, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        de::Deserializer::deserialize_map(self.prime, visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r#struct() {
        env_logger::builder().is_test(true).try_init().ok();
        #[derive(Debug, Deserialize, PartialEq)]
        struct Test {
            a: i32,
            b: Option<i32>,
            c: Option<String>,
            d: f64,
            e: Vec<Value>
        }
        let v = serde_json::from_str(
            r#"{ "o": [
            { "k": "a", "v": { "n": 3 } },
            {"k":"c","v":{"s":"sdf"}},
            {"k":"d","v":{"v":"Infinity"}},
            {"k": "e", "v": {"a": [{"n": 2.0}, {"b": false}]}}
            ] }"#
        )
        .unwrap();
        let de: Test = from_value(&v).unwrap();
        assert_eq!(
            de,
            Test {
                a: 3,
                b: None,
                c: Some("sdf".into()),
                d: f64::INFINITY,
                e: vec![Value::from(2.0f64), Value::from(false)]
            }
        );
        let v = serde_json::from_str(r#"{"v": "null"}"#).unwrap();
        let de: Option<String> = from_value(&v).unwrap();
        assert_eq!(de, None);
    }

    //#[test]
    // fn r#enum() {
    //    env_logger::builder().is_test(true).try_init().ok();
    //    #[derive(Debug, Deserialize, PartialEq)]
    //    enum Enum {
    //        A
    //    }
    //    let v = serde_json::from_str(r#"{"s": "a"}"#).unwrap();
    //    let de: Enum = from_value(&v).unwrap();
    //    assert_eq!(de, Enum::A);
    //}
}
