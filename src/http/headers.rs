use std::collections::HashMap;

#[derive(Debug)]
pub struct Headers {
    map: HashMap<String, String>,
}

impl Headers {
    pub fn new() -> Headers {
        return Headers {
            map: HashMap::new(),
        };
    }

    pub fn set_header(&mut self, name: &String, value: &String) {
        let name = name.to_ascii_lowercase();
        let value = value.to_ascii_lowercase();

        self.map.insert(name, value);
    }

    pub fn get_header(&self, name: &String) -> Option<&String> {
        let name = name.to_ascii_lowercase();

        self.map.get(&name)
    }

    pub fn get_map(&self) -> &HashMap<String, String> {
        &self.map
    }
}

impl PartialEq for Headers {
    fn eq(&self, other: &Headers) -> bool {
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
                    return true;
                }
                None => return false,
            })
            .filter(|val| !*val)
            .count()
            == 0
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
