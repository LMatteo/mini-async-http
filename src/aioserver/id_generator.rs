use std::collections::HashMap;

// Very basic implementation of a usize typed id generator
pub(crate) struct IdGenerator {
    used: HashMap<usize, ()>,
    pos: usize,
}

impl IdGenerator {
    pub fn new(start: usize) -> IdGenerator {
        IdGenerator {
            used: HashMap::new(),
            pos: start,
        }
    }

    pub fn id(&mut self) -> usize {
        loop {
            match self.used.get(&self.pos) {
                Some(_) => {
                    self.pos += 1;
                }
                None => {
                    self.used.insert(self.pos, ());
                    let id = self.pos;
                    self.pos += 1;
                    return id;
                }
            };
        }
    }

    pub fn remove(&mut self, id: usize) {
        self.used.remove(&id);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get_id() {
        let mut gen = IdGenerator::new(4);

        assert_ne!(gen.id(), gen.id());
    }
}
