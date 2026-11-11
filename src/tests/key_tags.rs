use crate::key_tags::{Side, tag_sides};

const WIDTH: f32 = 30.0;

fn sides(xs: &[f32]) -> Vec<Option<Side>> {
    tag_sides(xs, &vec![WIDTH; xs.len()], 0.0, 400.0)
}

#[test]
fn sparse_keys_put_tags_on_the_right() {
    assert_eq!(sides(&[20.0, 150.0, 250.0]), vec![Some(Side::Right); 3]);
}

#[test]
fn a_key_at_the_right_edge_puts_its_tag_on_the_left() {
    assert_eq!(sides(&[20.0, 400.0]), vec![Some(Side::Right), Some(Side::Left)]);
}

#[test]
fn a_key_at_the_left_edge_keeps_its_tag_on_the_right() {
    assert_eq!(sides(&[0.0]), vec![Some(Side::Right)]);
}

#[test]
fn a_tag_that_would_cover_the_next_key_moves_left_or_hides() {
    assert_eq!(sides(&[100.0, 120.0]), vec![Some(Side::Left), Some(Side::Right)]);
    assert_eq!(sides(&[10.0, 30.0, 50.0]), vec![None, None, Some(Side::Right)]);
}

#[test]
fn shown_tags_never_overlap_each_other_or_a_key() {
    let xs: Vec<f32> = (0..40).map(|i| i as f32 * 9.0 + (i % 3) as f32 * 4.0).collect();
    let placed = sides(&xs);
    let spans: Vec<(f32, f32)> = xs
        .iter()
        .zip(&placed)
        .filter_map(|(&x, side)| match side {
            Some(Side::Right) => Some((x + 8.0, x + 8.0 + WIDTH)),
            Some(Side::Left) => Some((x - 8.0 - WIDTH, x - 8.0)),
            None => None,
        })
        .collect();
    for pair in spans.windows(2) {
        assert!(pair[0].1 <= pair[1].0, "{pair:?}");
    }
    for (start, end) in &spans {
        assert!(xs.iter().all(|x| *x + 5.0 <= *start || *x - 5.0 >= *end));
    }
}
