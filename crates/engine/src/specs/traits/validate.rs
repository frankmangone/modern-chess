use crate::specs::GameSpecError;

pub trait Validate {
    type Arg1;
    type Arg2;

    /// A function to validate the contents of a spec.
    fn validate(&self, arg1: &Self::Arg1, arg2: &Self::Arg2) -> Result<(), GameSpecError>;
}