/* FGO Brave Chain Calculator
 *
 * TODO: crit%, np cards, >3 input cards, effectiveness multiplier, input number of hits, crit, overkill, star gen
 */

use std::collections::{HashMap};
use std::env;
use std::cmp;
use std::fmt;
use std::mem::{transmute};


#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum CardKind {
	Quick,
	Arts,
	Buster
}
impl fmt::Display for CardKind {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", match self { Quick => "Q", Arts => "A", Buster => "B" })
	}
}
use CardKind::*;

#[repr(usize)]
#[allow(dead_code)]
enum Position {
	First,
	Second,
	Third,
	Extra
}
use Position::*;

#[derive(Debug)]
struct HandStats {
	damage: f32,
	np: f32,
	stars: f32,
}

struct HandData {
	hand: Hand,
	data: HandStats,
}

impl fmt::Display for HandData {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}{}{}|dmg: {:5.0}, np: {:4.3}, stars {:3.1}",
			self.hand[0].to_string(), self.hand[1].to_string(), self.hand[2].to_string(),
			self.data.damage, self.data.np, self.data.stars)
	}
}

type Hand = [CardKind; 3];


fn chain_type(cards: &Hand) -> Option<CardKind> {
	let mut counts = [0, 0, 0];
	for &card in cards {
		counts[card as usize] += 1;
		if counts[card as usize] == 3 { return Some(card) }
	}
	None
}

fn card_stats(card: Option<CardKind>, position: Position) -> HandStats {
	let s = if let Some(kind) = card {
		match (kind, position) {
			(Buster, First)  => (1.5,  0.0, 0.10),
			(Buster, Second) => (1.8,  0.0, 0.15),
			(Buster, Third)  => (2.1,  0.0, 0.20),
			(Quick,  First)  => (0.8,  1.0, 0.80),
			(Quick,  Second) => (0.96, 1.5, 1.30),
			(Quick,  Third)  => (1.12, 2.0, 1.80),
			(Arts,   First)  => (1.0,  3.0, 0.0),
			(Arts,   Second) => (1.2,  4.5, 0.0),
			(Arts,   Third)  => (1.4,  6.0, 0.0),
			(_, Extra) => (-99999.01, -99999.01, -99999.01), // unreachable
		}
	} else {
		(1.0, 1.0, 1.0)
	};

	HandStats { damage: s.0, np: s.1, stars: s.2 }
}

fn hand_stats(cards: &Hand) -> HandStats {
	const SERVANT_ATTACK: f32 = 7000.0;
	const NUMBER_OF_HITS: f32 = 2.0;
	const NP_RATE: f32 = 0.01;
	const STAR_GENERATION: f32 = 0.1;

	let first_buster = if cards[First as usize] == Buster { 0.5 } else { 0.0 };

	let first_arts = if cards[First as usize] == Arts { 1.0 } else { 0.0 };

	let chain_type = chain_type(cards);

	let buster_chain = if chain_type == Some(Buster) { 0.2 } else { 0.0 };
	let quick_chain  = if chain_type == Some(Quick) { 0.2 } else { 0.0 };

	let card_stats = {
		let mut t: Vec<HandStats> = cards.iter().enumerate().map(
			|(idx, &card)| {
				let position = unsafe { transmute::<usize, Position>(idx) };
				card_stats(Some(card), position)
			}).collect();
		t.push(card_stats(None, Extra));
		t
	};

	let initial_values = HandStats
		{ damage: 0.0
		, np:     if chain_type == Some(Arts)  {  0.2 } else { 0.0 }
		, stars:  if chain_type == Some(Quick) { 10.0 } else { 0.0 } };

	card_stats.iter().enumerate().fold(initial_values,
		|acc, (i, x)| {
			let extra_card_modifier = if i == Extra as usize {
				if chain_type == None { 2.0 } else { 3.5 }
			} else {
				1.0
			};
			let damage = 0.23 * extra_card_modifier * SERVANT_ATTACK * (first_buster + x.damage) + SERVANT_ATTACK * buster_chain;

			let np = NUMBER_OF_HITS * NP_RATE * (first_arts + x.np);

			let stars = NUMBER_OF_HITS * (STAR_GENERATION + quick_chain + x.stars);

			HandStats {
				damage: acc.damage + damage,
				np:     acc.np     + np,
				stars:  acc.stars  + stars }
	})
}

fn translate(c: Option<char>) -> Result<CardKind, String> {
	match c {
		Some('a') => Ok(Arts),
		Some('b') => Ok(Buster),
		Some('q') => Ok(Quick),
		_ => Err(format!("Expected a/b/q. Found {:?}.", c))
	}
}

fn to_hand(query: &str) -> Result<Hand, String> {
	if query.len() != 3 { return Err(format!("Expected 3 cards. Found {} in \"{}\".", query.len(), query)) }

	let lower = query.to_lowercase();

	let mut s = lower.chars();

	let mut result = [Arts, Arts, Arts];

	for r in &mut result { *r = translate(s.next())? }

	Ok(result)
}

fn to_cards(hand: &[CardKind]) -> Hand {
	let mut result = [Arts, Arts, Arts];

	for (i, &card) in hand.iter().take(3).enumerate() { result[i] = card };

	result
}

fn clone_and_remove_item<T: PartialEq + Clone>(items: &[T], item: &T) -> Vec<T> {
	let mut copy = items.to_vec();

	for (i, x) in items.iter().enumerate() {
		if x == item {
			copy.remove(i);
			break
		}
	}
	copy
}

fn combinations(cards: &[CardKind]) -> impl Iterator<Item = Hand> {
	fn combinations_rec(cards: &[CardKind]) -> Vec<Vec<CardKind>> {
		if cards.is_empty() { return vec![Vec::new()] }

		let mut result = Vec::new();

		for card in cards {
			let mut hands = combinations_rec(&clone_and_remove_item(cards, card));

			for hand in &mut hands { hand.push(*card); }

			result.append(&mut hands);
		}
		result
	}
	combinations_rec(cards).into_iter().map(|x|to_cards(&x))
}

fn data(cards: &[CardKind]) -> HashMap<Hand, HandStats> {
	let mut result = HashMap::new();

	for hand in combinations(cards) {
		if result.contains_key(&hand) { continue }

		result.insert(hand, hand_stats(&hand));
	}

	result
}

fn main() {
	let query = env::args().nth(1).expect(
        &format!("Usage:\n\t{:?} <a/b/q><a/b/q><a/b/q>", env::args().next()));

	let mut hands: Vec<HandData> = match to_hand(&query) {
		Ok(hand) => data(&hand).into_iter().map(|x| HandData { hand:x.0, data:x.1 }).collect(),
		Err(err) => { println!("{}", err); return }
	};

	hands.sort_by(|a, b| { b.data.damage.partial_cmp(&a.data.damage).unwrap_or(cmp::Ordering::Equal) });
	println!("By Damage:");
	for hand in &hands { println!("{}", hand) }

	hands.sort_by(|a, b| { b.data.np.partial_cmp(&a.data.np).unwrap_or(cmp::Ordering::Equal) });
	println!("By NP:");
	for hand in &hands { println!("{}", hand) }
}
