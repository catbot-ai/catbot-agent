pub mod bar;
pub use bar::*;

pub mod predictions;
pub mod prices;
pub mod subscriptions;

pub use predictions::*;
pub use prices::*;
pub use subscriptions::*;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
