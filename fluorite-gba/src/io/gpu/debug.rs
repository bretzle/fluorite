#[derive(Debug)]
pub struct DebugSpecification {
    pub reg_enable: bool,
    pub map_enable: bool,
    pub tiles_enable: bool,
    pub palettes_enable: bool,

    pub map_spec: MapSpecification,
    pub tiles_spec: TilesSpecification,
}

impl DebugSpecification {
    pub fn new() -> Self {
        Self {
            reg_enable: true,
            map_enable: false,
            tiles_enable: false,
            palettes_enable: false,

            map_spec: MapSpecification::new(),
            tiles_spec: TilesSpecification::new(),
        }
    }
}

#[derive(Debug)]
pub struct MapSpecification {
    pub bg_i: usize,
}

impl MapSpecification {
    pub fn new() -> Self {
        Self { bg_i: 0 }
    }
}

#[derive(Debug)]
pub struct TilesSpecification {
    pub palette: i32,
    pub block: usize,
    pub bpp8: bool,
}

impl TilesSpecification {
    pub fn new() -> Self {
        Self {
            palette: 0,
            block: 0,
            bpp8: false,
        }
    }
}
