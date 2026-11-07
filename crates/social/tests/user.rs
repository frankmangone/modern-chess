use chrono::{TimeZone, Utc};
use modern_chess_kernel::Handle;
use modern_chess_social::User;

#[test]
fn guest_user_gets_the_given_handle_and_a_fresh_id() {
    let now = Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap();
    let handle = Handle::parse("Guest-4821").unwrap();

    let first = User::new_guest(handle.clone(), now);
    let second = User::new_guest(handle.clone(), now);

    assert_eq!(first.handle, handle);
    assert_eq!(first.created_at, now);
    assert_ne!(first.id, second.id);
}
