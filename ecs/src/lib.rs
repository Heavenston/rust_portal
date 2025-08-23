pub mod dynvec;
pub mod sparse_set;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    idx: u32,
    r#gen: u32,
}

pub struct World {
    
}
