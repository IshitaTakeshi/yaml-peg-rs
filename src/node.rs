use crate::*;
use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
    io::Result,
    ops::Index,
    slice::Iter,
    str::FromStr,
};

macro_rules! as_method {
    {$(#[$meta:meta])* fn $id:ident = $ty1:ident $(| $ty2:ident)*} => {
        $(#[$meta])*
        pub fn $id<N>(&self) -> Option<N>
        where
            N: FromStr,
        {
            match &self.yaml {
                Yaml::$ty1(n) $(| Yaml::$ty2(n))* => match n.parse() {
                    Ok(v) => Some(v),
                    Err(_) => None,
                },
                _ => None,
            }
        }
    };
}

macro_rules! except_method {
    {$(#[$meta:meta])* fn $id:ident = $ty1:ident $(| $ty2:ident)*} => {
        $(#[$meta])*
        pub fn $id<E, N>(&self, e: E) -> Result<N>
        where
            E: AsRef<str>,
            N: FromStr,
        {
            match match &self.yaml {
                Yaml::$ty1(n) $(| Yaml::$ty2(n))* => n,
                _ => "",
            }
            .parse()
            {
                Ok(v) => Ok(v),
                Err(_) => Err(err!(e.as_ref())),
            }
        }
    };
}

/// Parser node, includes line number, column number, type assertion and anchor.
///
/// This type will ignore additional members when comparison and hashing.
///
/// ```
/// use std::collections::HashSet;
/// use yaml_peg::Node;
/// let mut s = HashSet::new();
/// s.insert(Node::new("a".into()).pos(0));
/// s.insert(Node::new("a".into()).pos(1));
/// s.insert(Node::new("a".into()).pos(2));
/// assert_eq!(s.len(), 1);
/// ```
///
/// There is a convenient macro [`node!`] to create nodes literally.
///
/// Nodes can be indexing by `usize` or `&str`,
/// but it will always return self if the index is not contained.
///
/// ```
/// use yaml_peg::{Yaml, Node};
/// let node = Node::new(Yaml::Null);
/// assert_eq!(node["a"][0]["bc"], node);
/// ```
///
/// There are `as_*` methods provide `Option` returns,
/// default options can be created by [`Option::unwrap_or`].
///
/// In another hand, using `except_*` methods to convert the YAML types with **error** returns.
/// The `except_*` methods are support to use `null` as empty option (for user inputs).
#[derive(Eq, Clone)]
pub struct Node {
    /// Document position
    pub pos: usize,
    /// Type assertion
    pub ty: String,
    /// Anchor reference
    pub anchor: String,
    /// YAML data
    pub yaml: Yaml,
}

impl Node {
    /// Create node from YAML data.
    pub fn new(yaml: Yaml) -> Self {
        Self {
            pos: 0,
            ty: "".into(),
            anchor: "".into(),
            yaml,
        }
    }

    /// Builder function for position.
    pub fn pos(mut self, pos: usize) -> Self {
        self.pos = pos;
        self
    }

    /// Builder function for type assertion.
    pub fn ty(mut self, ty: String) -> Self {
        self.ty = ty;
        self
    }

    /// Builder function for anchor.
    pub fn anchor(mut self, anchor: String) -> Self {
        self.anchor = anchor;
        self
    }

    /// Check the value is null.
    pub fn is_null(&self) -> bool {
        if let Yaml::Null = self.yaml {
            true
        } else {
            false
        }
    }

    /// Convert to boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match &self.yaml {
            Yaml::Bool(b) => Some(*b),
            _ => None,
        }
    }

    as_method! {
        /// Convert to integer.
        fn as_int = Int
    }
    as_method! {
        /// Convert to float.
        fn as_float = Float
    }
    as_method! {
        /// Convert to number.
        fn as_number = Int | Float
    }

    /// Convert to array.
    ///
    /// Warn: The object ownership will be took.
    pub fn as_array(&self) -> Option<&Array> {
        match &self.yaml {
            Yaml::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Convert to map and try to get the value by keys.
    ///
    /// If get failed, returns [`Option::None`].
    pub fn as_get(&self, keys: &[&str]) -> Option<&Self> {
        if let Yaml::Map(a) = &self.yaml {
            get_from_map(a, keys)
        } else {
            None
        }
    }

    /// Assert the data is boolean.
    pub fn except_bool<E>(&self, e: E) -> Result<bool>
    where
        E: AsRef<str>,
    {
        match &self.yaml {
            Yaml::Bool(b) => Ok(*b),
            _ => Err(err!(e.as_ref())),
        }
    }

    except_method! {
        /// Assert the data is integer.
        ///
        /// If get failed, returns [`std::io::Error`].
        fn except_int = Int
    }
    except_method! {
        /// Assert the data is float.
        ///
        /// If get failed, returns [`std::io::Error`].
        fn except_float = Float
    }
    except_method! {
        /// Assert the data is float.
        ///
        /// If get failed, returns [`std::io::Error`].
        fn except_number = Int | Float
    }

    /// Assert the data is string reference.
    ///
    /// If get failed, returns [`std::io::Error`].
    /// Null value will generate an empty string.
    /// Warn: The object ownership will be took.
    pub fn except_str<E>(&self, e: E) -> Result<&str>
    where
        E: AsRef<str>,
    {
        match &self.yaml {
            Yaml::Str(s) => Ok(s.as_ref()),
            Yaml::Null => Ok(""),
            _ => Err(err!(e.as_ref())),
        }
    }

    /// Assert the data is string.
    ///
    /// If get failed, returns [`std::io::Error`].
    /// Null value will generate an empty string.
    pub fn except_string<E>(&self, e: E) -> Result<String>
    where
        E: AsRef<str>,
    {
        match &self.yaml {
            Yaml::Str(s) => Ok(s.clone()),
            Yaml::Null => Ok("".into()),
            _ => Err(err!(e.as_ref())),
        }
    }

    /// Assert the data is array.
    ///
    /// If get failed, returns [`std::io::Error`].
    /// Null value will generate an empty array.
    pub fn except_array<E>(&self, e: E) -> Result<(usize, Iter<Node>)>
    where
        E: AsRef<str>,
    {
        match &self.yaml {
            Yaml::Array(a) => Ok((a.len(), a.iter())),
            Yaml::Null => Ok((0, [].iter())),
            _ => Err(err!(e.as_ref())),
        }
    }

    /// Assert the data is map and try to get the value by keys.
    ///
    /// If get failed, returns [`std::io::Error`].
    pub fn except_get<E>(&self, keys: &[&str], e: E) -> Result<&Self>
    where
        E: AsRef<str>,
    {
        if let Yaml::Map(m) = &self.yaml {
            get_from_map(m, keys).ok_or(err!(e.as_ref()))
        } else {
            Err(err!(e.as_ref()))
        }
    }
}

fn get_from_map<'a>(m: &'a Map, keys: &[&str]) -> Option<&'a Node> {
    if keys.is_empty() {
        panic!("invalid search!");
    }
    let key = node!(keys[0].into());
    if let Some(v) = m.get(&key) {
        match &v.yaml {
            Yaml::Map(m) => {
                if keys[1..].is_empty() {
                    Some(v)
                } else {
                    get_from_map(m, &keys[1..])
                }
            }
            _ => Some(v),
        }
    } else {
        None
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("Node{:?}", &self.yaml))
    }
}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.yaml.hash(state)
    }
}

impl PartialEq for Node {
    fn eq(&self, rhs: &Self) -> bool {
        self.yaml == rhs.yaml
    }
}

impl Index<usize> for Node {
    type Output = Self;

    fn index(&self, index: usize) -> &Self::Output {
        match &self.yaml {
            Yaml::Array(a) => a.get(index).unwrap_or(self),
            Yaml::Map(m) => m
                .get(&Node::new(Yaml::Int(index.to_string())))
                .unwrap_or(self),
            _ => self,
        }
    }
}

impl Index<&str> for Node {
    type Output = Self;

    fn index(&self, index: &str) -> &Self::Output {
        if let Yaml::Map(m) = &self.yaml {
            m.get(&node!(index.into())).unwrap_or(self)
        } else {
            self
        }
    }
}
