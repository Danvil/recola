#[derive(Clone)]
pub struct FluidChunk<T> {
    pub volume: f64,
    pub data: T,
}
