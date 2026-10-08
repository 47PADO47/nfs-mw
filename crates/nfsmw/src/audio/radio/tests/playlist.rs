use super::super::playlist::{MAX_SONGS, Mode, Playlist, Random};

/// A "random" that always asks for the first, so shuffled order is predictable.
fn first(_count: usize) -> usize {
    0
}

fn take(list: &mut Playlist, mode: Mode, n: usize) -> Vec<usize> {
    let mut random = Random::new(7);
    (0..n).map(|_| list.next(mode, &mut |c| random.below(c)).unwrap()).collect()
}

#[test]
fn ordered_play_follows_the_list_and_wraps() {
    let mut list = Playlist::new([2, 5, 9]);
    assert_eq!((list.len(), list.contains(5), list.contains(4)), (3, true, false));
    assert_eq!(take(&mut list, Mode::Ordered, 7), [2, 5, 9, 2, 5, 9, 2]);
}

#[test]
fn songs_are_numbered_below_the_mask_width() {
    let list = Playlist::new([0, MAX_SONGS - 1, MAX_SONGS, 99]);
    assert_eq!(list.len(), 2);
    assert!(!list.contains(MAX_SONGS) && !list.contains(99));
}

#[test]
fn an_empty_list_plays_nothing() {
    let mut list = Playlist::new([]);
    assert!(list.is_empty());
    assert_eq!(list.next(Mode::Ordered, &mut first), None);
    assert_eq!(list.next(Mode::Shuffle, &mut first), None);
}

#[test]
fn a_single_song_repeats() {
    let mut list = Playlist::new([4]);
    assert_eq!(take(&mut list, Mode::Ordered, 3), [4, 4, 4]);
    assert_eq!(take(&mut list, Mode::Shuffle, 3), [4, 4, 4]);
}

#[test]
fn a_shuffle_plays_every_song_before_a_repeat() {
    let songs = [1, 3, 4, 8, 10, 13, 20];
    let mut list = Playlist::new(songs);
    let order = take(&mut list, Mode::Shuffle, 700);
    // The first round is a permutation of the list.
    let mut first_round = order[..songs.len()].to_vec();
    first_round.sort_unstable();
    assert_eq!(first_round, songs);
    // Never the same song twice in a row, and every song turns up in every 2N picks (a round leaves out the
    // song that ended the one before).
    assert!(order.windows(2).all(|w| w[0] != w[1]));
    for window in order.windows(2 * songs.len()) {
        assert!(songs.iter().all(|s| window.contains(s)), "{window:?}");
    }
}

#[test]
fn the_song_that_ends_a_round_is_left_out_of_the_next_round() {
    // With a "random" that always picks the first unplayed, the first round plays in list order. The song that
    // ended it (2) is missing from the second round, and the song that ends that round (1) from the third.
    let mut list = Playlist::new([0, 1, 2]);
    let order: Vec<usize> = (0..9).map(|_| list.next(Mode::Shuffle, &mut first).unwrap()).collect();
    assert_eq!(order, [0, 1, 2, 0, 1, 0, 2, 0, 1]);
}

#[test]
fn ordered_play_continues_after_the_last_song_even_if_a_mode_switch_happens() {
    let mut list = Playlist::new([0, 1, 2, 3]);
    assert_eq!(take(&mut list, Mode::Ordered, 2), [0, 1]);
    // Switching to shuffle plays the two that are left, then the round restarts.
    let mut rest = take(&mut list, Mode::Shuffle, 2);
    rest.sort_unstable();
    assert_eq!(rest, [2, 3]);
    assert_eq!(list.last().map(|n| n == 2 || n == 3), Some(true));
    assert_eq!(take(&mut list, Mode::Ordered, 1).len(), 1);
}

#[test]
fn reset_starts_the_round_over() {
    let mut list = Playlist::new([0, 1, 2]);
    assert_eq!(take(&mut list, Mode::Ordered, 2), [0, 1]);
    list.reset();
    assert_eq!(list.last(), None);
    assert_eq!(take(&mut list, Mode::Ordered, 1), [0]);
}

#[test]
fn the_generator_stays_in_range_and_moves() {
    let mut random = Random::from_clock();
    let values: Vec<usize> = (0..200).map(|_| random.below(7)).collect();
    assert!(values.iter().all(|&v| v < 7));
    assert!((0..7).all(|v| values.contains(&v)), "every value turns up in 200 draws");
    assert_eq!(random.below(0), 0);
    assert_eq!(random.below(1), 0);
}
