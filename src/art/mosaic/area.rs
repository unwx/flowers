use crate::math::area::Area;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AreaGroup {
    pub subgroups: Vec<Vec<Area>>,
}

impl AreaGroup {
    pub fn new(subgroups: Vec<Vec<Area>>) -> Self {
        Self { subgroups }
    }
}


pub trait AreaGroupDecorator {
    fn decorate_group(&self, group: AreaGroup, seed: u64) -> Result<AreaGroup>;
}


#[derive(Debug, Copy, Clone)]
pub struct NoopGroupDecorator;

impl AreaGroupDecorator for NoopGroupDecorator {
    fn decorate_group(&self, group: AreaGroup, _: u64) -> Result<AreaGroup> {
        Ok(group)
    }
}


#[derive(Debug, Copy, Clone)]
pub struct SeparateEachGroupDecorator;

impl AreaGroupDecorator for SeparateEachGroupDecorator {
    fn decorate_group(&self, group: AreaGroup, _: u64) -> Result<AreaGroup> {
        Ok(AreaGroup::new(
            group
                .subgroups
                .into_iter()
                .flatten()
                .map(|area| vec![area])
                .collect(),
        ))
    }
}
