# Discussion on a game's page

Status: **proposed** (28 Nov 2026), waiting for Frank's rulings on the three questions at the end. It is
step 2 of `2026-11-28-communities-design.md`: threads and posts on a game's page, with reports that
only staff see.

## What is decided already

From the communities design: every game page is a community; the talk lives on the social service,
keyed by the game page's slug; only a signed-in account that has chosen its handle can post, guests
read; reports go to a queue staff see, staff can hide a post, there are no moderators; replies are
flat; a game that is unpublished or removed keeps its discussion hidden with it. Decision 6 of the
ownership plan adds: no automatic takedown.

## What a person sees

On a game's page, under the lobbies and the visitor's record, a **Discussion** section: the five
threads with the latest activity (title, who started it, how many replies, how long ago) and "All
discussion". A guest sees the threads and a line, "Sign in to join in". A signed-in person sees
"Start a thread".

`/games/:slug/discussion` is the full list, newest activity first, twenty at a time. A thread's page,
`/games/:slug/discussion/:thread`, is its title, the opening post and the replies in the order they
were written, with a box to reply. Each post shows its author's handle (and their record on this game,
since it is public and already there), the time, "Report", and "Delete" on your own.

Posts are **plain text**: line breaks are kept, nothing is formatted, and a link is shown as text and
is not clickable. That is the safe default for a place with no moderators; links can be turned on
later and cannot easily be taken back.

## The data (social service, migration `0004`)

- **`threads`**: `id`, `game_slug`, `author_id`, `title` (3 to 120 characters), `created_at`,
  `last_activity_at`. The thread's body is its first post, so a thread is never empty.
- **`posts`**: `id`, `thread_id`, `author_id`, `body` (1 to 5000 characters), `created_at`, `status`.
- **`reports`**: `id`, `post_id`, `reporter_id`, `reason`, `note` (up to 500 characters), `created_at`,
  `status`, `resolved_by`, `resolved_at`. One report for each person on each post.
- **`post_status_history`**: who, when, from, to, reason, written with every change of a post's status.

`author_id` is the opaque user id, as everywhere. The handle shown is read from `users` at the time,
so a person who deleted their account appears as `deleted-<hex>`, as the accounts plan already says,
and their posts stay (a thread with holes in it is worse than one with a nameless voice).

### State machines

Following the working rule in the ownership plan: one enum, one pure `transition`, a trigger in the
database that refuses an illegal move, the table tested pair by pair.

**Post** (`status`):

| From | To | Who |
|------|----|-----|
| `visible` | `hidden` | staff (for a report, or on their own look) |
| `visible` | `deleted` | the author |
| `hidden` | `visible` | staff (a reinstatement) |

`deleted` and a staff-hidden post both show as a grey line in the thread ("Deleted by its author",
"Hidden by staff") with the body not sent to anyone, so the replies still make sense. A `deleted` post
never comes back, and a staff-hidden one cannot be deleted by its author (they could otherwise
escape a reinstatement). Hiding a thread's opening post takes the thread out of the lists; its replies
stay with it, unreachable by the list.

**Report** (`status`): `open` to `actioned` (staff hid the post) or to `dismissed` (staff left it).
Closed for good. Hiding a post closes every open report on it as `actioned`.

No automatic hiding after any number of reports. A report is a message to staff, not a vote.

## Who may do what

| Action | Who |
|--------|-----|
| Read threads and posts | anyone, guests included |
| Start a thread, reply, report a post | a signed-in account with its handle chosen |
| Delete a post | its author |
| Read the report queue, hide or reinstate a post, dismiss a report | staff only |

"Signed in with a handle chosen" is the token's `account` flag plus `setup_pending` being false, read
from the user row, not from the token. Staff is the same `STAFF_USER_IDS` list the game service reads,
given to the social service as well and nothing else: for anyone else the staff routes answer "not
found". Nobody reports their own post.

Small limits so a bad actor costs little: a person starts at most one thread a minute and posts at
most one reply every ten seconds (checked against the last post's time, so no counter to keep), and a
thread takes up to a thousand posts. Reasons for a report: spam, abuse, off topic, illegal, other.

## Which games have a discussion (and the boundary)

A discussion belongs to a game page, but the social service has no games, and it must not take a
thread for a page that does not exist or one that is unpublished. Options:

- **A. Social asks the game service.** A small port in the social crate, `GamePages::is_open(slug)`,
  implemented by a call to the game service's public page endpoint (`GET /api/game/catalog/{slug}`),
  answered from a short cache (a minute). 200 means open; 404 means no page or not published. The
  address comes from one new setting, `SOCIAL_GAME_API_URL`. The game service learns nothing about
  discussion, and unpublishing hides the talk by itself, without a message between the services.
- B. The game service tells social when a page appears or changes. More moving parts, and a message
  that can be lost.
- C. Social trusts the slug. Anyone could make threads on any string.

**Recommended: A.** It adds one outgoing read, keeps the services independent and uses what the game
service already says in public. The cost: the social service depends on the game service being up to
post (reads of threads on a page the game service cannot answer for also fail closed, which is the
safe side), and a minute's lag between unpublishing and the talk disappearing.

## Routes (social service, under `/api/social`)

- `GET /games/{slug}/threads?before=<cursor>`; `POST /games/{slug}/threads` (`title`, `body`)
- `GET /threads/{id}?after=<cursor>`; `POST /threads/{id}/posts` (`body`)
- `DELETE /posts/{id}` (the author); `POST /posts/{id}/reports` (`reason`, `note`)
- staff: `GET /reports?status=open`, `POST /reports/{id}/dismiss`, `POST /posts/{id}/hide`,
  `POST /posts/{id}/reinstate`

## Slices

Each its own branch and PR.

1. **Threads and posts, read and write** (server): migration, the post machine, the game check, the
   limits, tests including the machine pair by pair.
2. **The web part**: the section on the game's page, the list page, the thread page, with the sign-in
   line for guests and the grey lines for hidden and deleted posts.
3. **Reports and the staff queue** (server and web): reporting, the queue as a section of the existing
   `/review` page, hide, reinstate and dismiss.

## Not built

Editing a post, votes, mentions and notifications, search, nested replies, locking or pinning a
thread, links, images, and moderators other than staff. Each is easy to add later and is left out on
purpose.

## Questions for Frank

1. **How does social know a page exists and is open?** A (it asks the game service, recommended), B or C
   above.
2. **May a person delete their own post, and edit it?** Proposed: delete yes (it leaves a grey line),
   edit not yet. Editing means keeping the old text for staff to read when a report arrives, which is
   a small design of its own.
3. **Plain text only, links not clickable?** Proposed yes, as above.

The limits, the reasons for a report, no automatic hiding and the three slices are defaults I took;
say if any is wrong.
