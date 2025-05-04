// Enum for the different types of prediction requests
#[derive(Debug, Clone)]
pub enum PredictionType {
    Trading,
    Graph,
    Rebalance,
}
