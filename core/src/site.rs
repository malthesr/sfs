use crate::Scs;

mod project;
pub use project::{Projection, ProjectionError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Site {
    count: Vec<usize>,
    projected: Projected,
}

impl Site {
    pub fn count(&self) -> &[usize] {
        &self.count
    }

    pub fn count_mut(&mut self) -> &mut [usize] {
        &mut self.count
    }

    pub fn new_projected(projection: Projection) -> Self {
        let dimensions = projection.dimensions();

        Self {
            count: vec![0; dimensions],
            projected: Projected::Projected(projection),
        }
    }

    pub fn new_unprojected(dimensions: usize) -> Self {
        Self {
            count: vec![0; dimensions],
            projected: Projected::Unprojected,
        }
    }

    pub fn try_add_to(&mut self, scs: &mut Scs) -> Result<(), ProjectionError> {
        match &mut self.projected {
            Projected::Projected(projection) => projection.project_to(&self.count, scs),
            Projected::Unprojected => {
                scs[&self.count] += 1.0;
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Projected {
    Projected(Projection),
    Unprojected,
}
