use glam::Affine3A;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapItemKind {
    Light,
    ObservationRoom,
    Entrance,
    Exit,
    Button,
}

#[derive(Debug, Clone)]
pub struct MapItem {
    pub kind: MapItemKind,
    pub transform: Affine3A,
}

impl MapItem {
    
}
