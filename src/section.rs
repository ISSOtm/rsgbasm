#[derive(Debug)]
pub enum Type {
    Rom0,
    Romx,
    Vram,
    Sram,
    Wram0,
    Wramx,
    Oam,
    Hram,
}

#[derive(Debug)]
struct Attrs {
    field: Type,
}
