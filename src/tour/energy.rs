//! The machine recovers energy on a completed recipe and exposes a saved buffer.
use mod_api::*;

pub const CAPACITY: i32 = 1_000;
const PER_RECIPE: i32 = 25;

pub fn buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: CAPACITY,
        max_insert: 100,
        max_extract: 100,
        insert_faces: [true; 6],
        extract_faces: [true; 6],
    }
}

pub fn charged(previous: Option<&Tag>) -> Tag {
    let amount = match previous {
        Some(Tag::Int(amount)) => *amount,
        _ => 0,
    };
    Tag::Int(
        amount
            .clamp(0, CAPACITY)
            .saturating_add(PER_RECIPE)
            .min(CAPACITY),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_energy_is_saved_and_bounded() {
        assert_eq!(charged(None), Tag::Int(25));
        assert_eq!(charged(Some(&Tag::Int(990))), Tag::Int(CAPACITY));
        assert_eq!(charged(Some(&Tag::Int(CAPACITY))), Tag::Int(CAPACITY));
        assert_eq!(buffer().capacity, CAPACITY);
    }
}
