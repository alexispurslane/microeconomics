#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub enum Item {
    FoodUnit,
    HouseUnit,
    LeisureUnit1,
    LeisureUnit2,
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone)]
pub enum Goal {
    Eat,
    Shelter,
    Rest,
    Leisure,
}
