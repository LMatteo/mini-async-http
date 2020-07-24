use std::collections::hash_map;
use std::collections::HashMap;

/// The HTTP header map.
/// All the names are not case sensitive.
///
/// # Example
///
/// ```
/// let mut headers = mini_async_http::Headers::new();
///
/// assert!(headers.get_header("missing").is_none());
///
/// headers.set_header("Content-type","text/plain");
/// assert_eq!(headers.get_header("content-type").unwrap(),"text/plain");
///
/// headers.set_header("Content-type","application/json");
/// assert_eq!(headers.get_header("content-type").unwrap(),"application/json");
/// ```
#[derive(Debug, Clone)]
pub struct Headers {
    map: HashMap<String, String>,
}

impl Headers {
    /// Init an empty header struct
    pub fn new() -> Headers {
        Headers {
            map: HashMap::new(),
        }
    }

    /// Set the given header name to the given value. If the key already exists overwrite the value.
    pub fn set_header(&mut self, name: &str, value: &str) {
        let name = name.to_ascii_lowercase();
        let value = value.to_ascii_lowercase();

        self.map.insert(name, value);
    }

    /// Retrieve the value at the given key
    pub fn get_header(&self, name: &str) -> Option<&String> {
        let name = name.to_ascii_lowercase();

        self.map.get(&name)
    }

    /// Return an iterator over all the headers. All keys are lowercase
    pub fn iter(&self) -> HeaderIterator {
        HeaderIterator {
            inner: self.map.iter(),
        }
    }
}

impl PartialEq for Headers {
    fn eq(&self, other: &Headers) -> bool {
        if self.map == other.map {
            return true;
        }

        if self.map.len() != other.map.len() {
            return false;
        }

        self.map
            .iter()
            .map(|(key, value)| match other.get_header(key) {
                Some(val) => {
                    if val != value {
                        return false;
                    }
                    true
                }
                None => false,
            })
            .filter(|val| !*val)
            .count()
            == 0
    }
}

impl Default for Headers {
    fn default() -> Self {
        Headers::new()
    }
}

impl IntoIterator for Headers {
    type Item = (String, String);
    type IntoIter = hash_map::IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

pub struct HeaderIterator<'a> {
    inner: hash_map::Iter<'a, String, String>,
}

impl<'a> Iterator for HeaderIterator<'a> {
    type Item = (&'a String, &'a String);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a> ExactSizeIterator for HeaderIterator<'a> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn eq() {
        let a = Headers::new();
        let b = Headers::new();

        assert_eq!(a, b)
    }

    #[test]
    fn key_eq() {
        let mut a = Headers::new();
        let mut b = Headers::new();

        a.set_header(&String::from("key"), &String::from("val"));
        a.set_header(&String::from("Content_length"), &String::from("89"));
        b.set_header(&String::from("KEY"), &String::from("val"));
        b.set_header(&String::from("Content_length"), &String::from("89"));

        assert_eq!(a, b)
    }

    #[test]
    fn not_eq() {
        let mut a = Headers::new();
        let b = Headers::new();

        a.set_header(&String::from("key"), &String::from("val"));

        assert_ne!(a, b)
    }

    #[test]
    fn not_eq_longer() {
        let mut a = Headers::new();
        let mut b = Headers::new();

        a.set_header(&String::from("key"), &String::from("val"));
        a.set_header(&String::from("Content_length"), &String::from("89"));
        b.set_header(&String::from("KEY"), &String::from("val"));
        b.set_header(&String::from("Content_length"), &String::from("89"));
        b.set_header(&String::from("diff"), &String::from("diff"));

        assert_ne!(a, b)
    }

    #[test]
    fn not_eq_val() {
        let mut a = Headers::new();
        let mut b = Headers::new();

        a.set_header(&String::from("key"), &String::from("valdiff"));
        a.set_header(&String::from("Content_length"), &String::from("89"));
        b.set_header(&String::from("KEY"), &String::from("val"));
        b.set_header(&String::from("Content_length"), &String::from("89"));

        assert_ne!(a, b)
    }
}
