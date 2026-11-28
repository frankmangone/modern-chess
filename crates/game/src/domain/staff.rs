//! Who is staff.
//!
//! There are no roles yet (they come with communities). A person is staff when their id is in the
//! list the server was started with, and in no other way: not from a token, not from the database,
//! and not on anyone's word. [`Staff::actor_for`] is the one place an [`Actor`] is made for a
//! person, so a route cannot hand out the staff's powers by mistake.

use std::collections::HashSet;

use modern_chess_kernel::UserId;

use super::Actor;

/// The people who may do what only staff may do.
#[derive(Debug, Clone, Default)]
pub struct Staff {
    ids: HashSet<UserId>,
}

impl Staff {
    pub fn new(ids: impl IntoIterator<Item = UserId>) -> Self {
        Self {
            ids: ids.into_iter().collect(),
        }
    }

    pub fn is_staff(&self, user: UserId) -> bool {
        self.ids.contains(&user)
    }

    /// The part `user` plays in a move on a game whose author is `author`, or none: a stranger has
    /// no part, and the caller answers "not found". Staff acting on their own game are its author,
    /// not staff: nobody approves their own work.
    pub fn actor_for(&self, user: UserId, author: UserId) -> Option<Actor> {
        if user == author {
            Some(Actor::Author)
        } else if self.is_staff(user) {
            Some(Actor::Staff)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_those_listed_are_staff_and_nobody_is_by_default() {
        let (ada, grace) = (UserId::new(), UserId::new());
        let staff = Staff::new([ada]);
        assert!(staff.is_staff(ada));
        assert!(!staff.is_staff(grace));
        assert!(!Staff::default().is_staff(ada));
    }

    #[test]
    fn staff_acting_on_their_own_game_are_its_author() {
        let (ada, grace) = (UserId::new(), UserId::new());
        let staff = Staff::new([ada]);
        assert_eq!(staff.actor_for(ada, ada), Some(Actor::Author));
        assert_eq!(staff.actor_for(ada, grace), Some(Actor::Staff));
    }

    #[test]
    fn a_stranger_has_no_part_in_someone_elses_game() {
        let (ada, grace, linus) = (UserId::new(), UserId::new(), UserId::new());
        assert_eq!(Staff::new([ada]).actor_for(grace, linus), None);
    }
}
