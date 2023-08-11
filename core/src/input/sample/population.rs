use indexmap::IndexSet;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Population {
    Named(String),
    Unnamed,
}

impl<S> From<Option<S>> for Population
where
    S: ToString,
{
    fn from(population: Option<S>) -> Self {
        match population {
            Some(population) => Self::Named(population.to_string()),
            None => Self::Unnamed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Id(pub usize);

impl From<Id> for usize {
    fn from(id: Id) -> Self {
        id.0
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Map(IndexSet<Population>);

impl Map {
    pub fn get(&self, name: &Population) -> Option<Id> {
        self.0.get_index_of(name).map(Id)
    }

    pub fn get_or_insert(&mut self, name: Population) -> Id {
        self.get(&name).unwrap_or_else(|| self.insert(name))
    }

    pub fn insert(&mut self, name: Population) -> Id {
        Id(self.0.insert_full(name).0)
    }
}
